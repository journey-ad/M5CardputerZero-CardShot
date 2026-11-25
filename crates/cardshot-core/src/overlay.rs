//! 屏幕提示条：写入帧缓冲；录制与截图时把提示区域还原
use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

const WIDTH: usize = 320;
const STRIDE: usize = WIDTH * 2;
const TOAST_X: usize = (WIDTH - 148) / 2;
const TOAST_Y: usize = 6;

#[derive(Clone)]
struct Mask {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    rows: Vec<Vec<u8>>,
}

static MASK: OnceLock<Mutex<Option<Mask>>> = OnceLock::new();

fn mask() -> &'static Mutex<Option<Mask>> {
    MASK.get_or_init(|| Mutex::new(None))
}

fn rgb8_to_rgb565_rows(rgb: &[u8], width: usize, height: usize) -> Vec<Vec<u8>> {
    let mut rows = Vec::with_capacity(height);
    for y in 0..height {
        let mut row = vec![0u8; width * 2];
        for x in 0..width {
            let byte_index = (y * width + x) * 3;
            let r = rgb[byte_index] as u16 >> 3;
            let g = rgb[byte_index + 1] as u16 >> 2;
            let b = rgb[byte_index + 2] as u16 >> 3;
            let pixel = (r << 11) | (g << 5) | b;
            row[x * 2] = (pixel & 0xff) as u8;
            row[x * 2 + 1] = (pixel >> 8) as u8;
        }
        rows.push(row);
    }
    rows
}

fn read_rows(
    file: &File,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> std::io::Result<Vec<Vec<u8>>> {
    let mut rows = Vec::with_capacity(height);
    for row_index in 0..height {
        let mut row = vec![0u8; width * 2];
        file.read_exact_at(&mut row, ((y + row_index) * STRIDE + x * 2) as u64)?;
        rows.push(row);
    }
    Ok(rows)
}

fn write_rows(file: &File, x: usize, y: usize, rows: &[Vec<u8>]) -> std::io::Result<()> {
    for (row_index, row) in rows.iter().enumerate() {
        file.write_all_at(row, ((y + row_index) * STRIDE + x * 2) as u64)?;
    }
    Ok(())
}

/// 显示一张提示图（PNG 字节）约 1 秒；期间录制线程会把该区域还原成原始画面
pub fn toast(fb_path: &str, png: &[u8]) -> std::io::Result<()> {
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?
        .to_rgb8();
    let (width, height) = (img.width() as usize, img.height() as usize);
    let rows = rgb8_to_rgb565_rows(img.as_raw(), width, height);
    let file = OpenOptions::new().read(true).write(true).open(fb_path)?;
    let saved = read_rows(&file, TOAST_X, TOAST_Y, width, height)?;
    *mask().lock().unwrap() = Some(Mask {
        x: TOAST_X,
        y: TOAST_Y,
        width,
        height,
        rows: saved.clone(),
    });
    // 提示条每 30 毫秒重绘一次，持续约 1 秒
    for _ in 0..34 {
        let _ = write_rows(&file, TOAST_X, TOAST_Y, &rows);
        thread::sleep(Duration::from_millis(30));
    }
    let _ = write_rows(&file, TOAST_X, TOAST_Y, &saved);
    *mask().lock().unwrap() = None;
    Ok(())
}

/// 在采集到的一帧（RGB565，宽 320）里把提示区域替换回原画面
pub fn apply_mask(frame: &mut [u8]) {
    let guard = mask().lock().unwrap();
    if let Some(mask) = guard.as_ref() {
        for (row_index, row) in mask.rows.iter().enumerate() {
            let y = mask.y + row_index;
            if y >= 170 {
                break;
            }
            let off = y * STRIDE + mask.x * 2;
            let end = (off + mask.width * 2).min(frame.len());
            let count = end.saturating_sub(off);
            frame[off..off + count].copy_from_slice(&row[..count]);
        }
    }
}
