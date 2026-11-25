use crate::util::{expand_home, local_time_fmt};
use image::{Rgb, RgbImage};
use std::fs::File;
use std::io::{Result, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const W: u32 = 320;
pub const H: u32 = 170;
pub const FRAME_BYTES: usize = (W as usize) * (H as usize) * 2;

pub const FB_PATH: &str = "/dev/fb0";

pub fn read_frame() -> Result<Vec<u8>> {
    let mut file = File::open(FB_PATH)?;
    let mut buf = vec![0u8; FRAME_BYTES];
    use std::io::Read;
    file.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn frame_to_rgb(raw: &[u8]) -> RgbImage {
    let mut img = RgbImage::new(W, H);
    for y in 0..H {
        for x in 0..W {
            let byte_index = ((y * W + x) as usize) * 2;
            if byte_index + 1 >= raw.len() {
                continue;
            }
            let pixel = u16::from_le_bytes([raw[byte_index], raw[byte_index + 1]]);
            let r = ((pixel >> 11) & 0x1f) as u32;
            let g = ((pixel >> 5) & 0x3f) as u32;
            let b = (pixel & 0x1f) as u32;
            img.put_pixel(
                x,
                y,
                Rgb([
                    (r * 255 / 31) as u8,
                    (g * 255 / 63) as u8,
                    (b * 255 / 31) as u8,
                ]),
            );
        }
    }
    img
}

pub fn ext_for(fmt: &str) -> &'static str {
    match fmt {
        "jpeg" | "jpg" => "jpg",
        "webp" => "webp",
        _ => "png",
    }
}

pub fn save_frame(raw: &[u8], path: &Path, fmt: &str, quality: u8) -> Result<()> {
    if let Some(parent_dir) = path.parent() {
        std::fs::create_dir_all(parent_dir).ok();
    }
    match fmt {
        "jpeg" | "jpg" => {
            let img = frame_to_rgb(raw);
            let file = File::create(path)?;
            let mut enc =
                image::codecs::jpeg::JpegEncoder::new_with_quality(file, quality.clamp(1, 100));
            enc.encode_image(&img).map_err(|error| {
                std::io::Error::new(std::io::ErrorKind::Other, error.to_string())
            })?;
        }
        "webp" => {
            save_webp(raw, path, quality)?;
        }
        _ => {
            let img = frame_to_rgb(raw);
            img.save_with_format(path, image::ImageFormat::Png)
                .map_err(|error| {
                    std::io::Error::new(std::io::ErrorKind::Other, error.to_string())
                })?;
        }
    }
    Ok(())
}

fn save_webp(raw: &[u8], path: &Path, quality: u8) -> Result<()> {
    let quality_text = quality.clamp(1, 100).to_string();
    let mut child = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb565le",
            "-s",
            "320x170",
            "-i",
            "-",
            "-frames:v",
            "1",
            "-c:v",
            "libwebp",
            "-quality",
            &quality_text,
        ])
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(raw);
    }
    let st = child.wait()?;
    if !st.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "ffmpeg webp failed",
        ));
    }
    Ok(())
}

pub fn screenshot(output_dir: &str, fmt: &str, quality: u8) -> Result<PathBuf> {
    let raw = read_frame()?;
    let dir = expand_home(output_dir);
    std::fs::create_dir_all(&dir)?;
    let name = format!("shot_{}.{}", local_time_fmt("%Y%m%d_%H%M%S"), ext_for(fmt));
    let path = dir.join(name);
    save_frame(&raw, &path, fmt, quality)?;
    Ok(path)
}
