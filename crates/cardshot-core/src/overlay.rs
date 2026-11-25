//! 屏幕提示条：直接写入帧缓冲，并在录制/截图时把该区域还原，保证提示不出现在成像结果里
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
    w: usize,
    h: usize,
    rows: Vec<Vec<u8>>,
}

static MASK: OnceLock<Mutex<Option<Mask>>> = OnceLock::new();

fn mask() -> &'static Mutex<Option<Mask>> {
    MASK.get_or_init(|| Mutex::new(None))
}

fn rgb8_to_rgb565_rows(rgb: &[u8], w: usize, h: usize) -> Vec<Vec<u8>> {
    let mut rows = Vec::with_capacity(h);
    for y in 0..h {
        let mut row = vec![0u8; w * 2];
        for x in 0..w {
            let i = (y * w + x) * 3;
            let r = rgb[i] as u16 >> 3;
            let g = rgb[i + 1] as u16 >> 2;
            let b = rgb[i + 2] as u16 >> 3;
            let v = (r << 11) | (g << 5) | b;
            row[x * 2] = (v & 0xff) as u8;
            row[x * 2 + 1] = (v >> 8) as u8;
        }
        rows.push(row);
    }
    rows
}

fn read_rows(file: &File, x: usize, y: usize, w: usize, h: usize) -> std::io::Result<Vec<Vec<u8>>> {
    let mut rows = Vec::with_capacity(h);
    for r in 0..h {
        let mut row = vec![0u8; w * 2];
        file.read_exact_at(&mut row, ((y + r) * STRIDE + x * 2) as u64)?;
        rows.push(row);
    }
    Ok(rows)
}

fn write_rows(file: &File, x: usize, y: usize, rows: &[Vec<u8>]) -> std::io::Result<()> {
    for (r, row) in rows.iter().enumerate() {
        file.write_all_at(row, ((y + r) * STRIDE + x * 2) as u64)?;
    }
    Ok(())
}

/// 显示一张提示图（PNG 字节）约 1 秒；期间录制线程会把该区域还原成原始画面
pub fn toast(fb_path: &str, png: &[u8]) -> std::io::Result<()> {
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        .to_rgb8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let rows = rgb8_to_rgb565_rows(img.as_raw(), w, h);
    let file = OpenOptions::new().read(true).write(true).open(fb_path)?;
    let saved = read_rows(&file, TOAST_X, TOAST_Y, w, h)?;
    *mask().lock().unwrap() = Some(Mask { x: TOAST_X, y: TOAST_Y, w, h, rows: saved.clone() });
    // 前台应用可能持续整帧重绘，提示需要周期性重绘才能保持可见
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
    if let Some(m) = guard.as_ref() {
        for (r, row) in m.rows.iter().enumerate() {
            let y = m.y + r;
            if y >= 170 {
                break;
            }
            let off = y * STRIDE + m.x * 2;
            let end = (off + m.w * 2).min(frame.len());
            let n = end.saturating_sub(off);
            frame[off..off + n].copy_from_slice(&row[..n]);
        }
    }
}
