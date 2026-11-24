use crate::config::Config;
use crate::util::expand_home;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

#[derive(Serialize, Clone)]
pub struct Item {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub size: u64,
    pub mtime: f64,
    pub duration: Option<f64>,
}

pub struct Library {
    pub output_dir: PathBuf,
    pub video_dir: PathBuf,
    pub thumb_dir: PathBuf,
    dur_cache: Mutex<HashMap<String, f64>>,
}

struct Raw {
    id: String,
    kind: String,
    name: String,
    size: u64,
    mtime: f64,
    path: PathBuf,
}

fn scan_dir(dir: &Path, kind: &str, exts: &[&str], out: &mut Vec<Raw>) {
    let rd = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let name = match p.file_name().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let ext = p
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !exts.contains(&ext.as_str()) {
            continue;
        }
        let md = match e.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let mtime = md
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        out.push(Raw {
            id: format!("{}:{}", kind, name),
            kind: kind.into(),
            name,
            size: md.len(),
            mtime,
            path: p,
        });
    }
}

impl Library {
    pub fn new(cfg: &Config) -> Self {
        Library {
            output_dir: expand_home(&cfg.output_dir),
            video_dir: expand_home(&cfg.video_dir),
            thumb_dir: expand_home(&cfg.thumb_dir),
            dur_cache: Mutex::new(HashMap::new()),
        }
    }

    fn scan(&self) -> Vec<Raw> {
        let mut v = Vec::new();
        scan_dir(&self.output_dir, "image", &["png", "jpg", "jpeg", "webp"], &mut v);
        scan_dir(&self.video_dir, "video", &["mp4", "mkv", "webm", "mov"], &mut v);
        v.sort_by(|a, b| b.mtime.partial_cmp(&a.mtime).unwrap_or(std::cmp::Ordering::Equal));
        v
    }

    fn duration_cached(&self, r: &Raw) -> f64 {
        let key = format!("{}|{}", r.path.to_string_lossy(), r.mtime);
        {
            let cache = self.dur_cache.lock().unwrap();
            if let Some(d) = cache.get(&key) {
                return *d;
            }
        }
        let d = duration_of(&r.path).unwrap_or(0.0);
        self.dur_cache.lock().unwrap().insert(key, d);
        d
    }

    /// 轻量统计：图片数、视频数、总字节（不调用 ffprobe）
    pub fn stats(&self) -> (usize, usize, u64) {
        let raw = self.scan();
        let im = raw.iter().filter(|r| r.kind == "image").count();
        let vi = raw.len() - im;
        let bytes = raw.iter().map(|r| r.size).sum();
        (im, vi, bytes)
    }

    pub fn items(&self) -> Vec<Item> {
        self.scan()
            .into_iter()
            .map(|r| {
                let duration = if r.kind == "video" {
                    Some(self.duration_cached(&r))
                } else {
                    None
                };
                Item {
                    id: r.id,
                    kind: r.kind,
                    name: r.name,
                    size: r.size,
                    mtime: r.mtime,
                    duration,
                }
            })
            .collect()
    }

    pub fn resolve(&self, id: &str) -> Option<(String, String, PathBuf)> {
        let (kind, name) = id.split_once(':')?;
        let dir = if kind == "video" {
            &self.video_dir
        } else {
            &self.output_dir
        };
        let p = dir.join(name);
        if p.is_file() {
            Some((kind.to_string(), name.to_string(), p))
        } else {
            None
        }
    }

    fn thumb_key(&self, it: &Item) -> u64 {
        let mut h = DefaultHasher::new();
        it.id.hash(&mut h);
        it.mtime.to_bits().hash(&mut h);
        it.size.hash(&mut h);
        h.finish()
    }

    pub fn ensure_thumb(&self, it: &Item) -> Option<PathBuf> {
        fs::create_dir_all(&self.thumb_dir).ok()?;
        let key = self.thumb_key(it);
        let dst = self.thumb_dir.join(format!("{:016x}.png", key));
        if dst.is_file() {
            return Some(dst);
        }
        let (_, _, src) = self.resolve(&it.id)?;
        let img = if it.kind == "video" {
            let tmp = self.thumb_dir.join(format!("{:016x}_src.png", key));
            if !video_frame(&src, &tmp) {
                return None;
            }
            let im = image::open(&tmp).ok()?;
            let _ = fs::remove_file(&tmp);
            im
        } else {
            image::open(&src).ok()?
        };
        let tw = 240u32;
        let ratio = tw as f32 / img.width().max(1) as f32;
        let th = (img.height() as f32 * ratio).max(1.0) as u32;
        let thumb = img.resize_exact(tw, th, image::imageops::FilterType::Lanczos3);
        thumb
            .save_with_format(&dst, image::ImageFormat::Png)
            .ok()?;
        Some(dst)
    }

    pub fn delete(&self, ids: &[String]) -> usize {
        let mut n = 0;
        for id in ids {
            if let Some((_, _, p)) = self.resolve(id) {
                if fs::remove_file(&p).is_ok() {
                    n += 1;
                }
            }
        }
        n
    }
}

fn video_frame(src: &Path, dst: &Path) -> bool {
    for ss in ["1", "0"] {
        let st = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y", "-ss", ss, "-i"])
            .arg(src)
            .args(["-frames:v", "1"])
            .arg(dst)
            .status();
        if let Ok(s) = st {
            if s.success() && dst.is_file() {
                return true;
            }
        }
    }
    false
}

fn duration_of(p: &Path) -> Option<f64> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=nw=1:nk=1",
        ])
        .arg(p)
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    s.trim().parse::<f64>().ok()
}
