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
    dur_file: PathBuf,
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
    let read_dir_result = match fs::read_dir(dir) {
        Ok(entry) => entry,
        Err(_) => return,
    };
    for dir_entry in read_dir_result.flatten() {
        let path = dir_entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|text| text.to_str()) {
            Some(text) => text.to_string(),
            None => continue,
        };
        let ext = path
            .extension()
            .and_then(|text| text.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !exts.contains(&ext.as_str()) {
            continue;
        }
        let metadata = match dir_entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration_secs| duration_secs.as_secs_f64())
            .unwrap_or(0.0);
        out.push(Raw {
            id: format!("{}:{}", kind, name),
            kind: kind.into(),
            name,
            size: metadata.len(),
            mtime,
            path: path,
        });
    }
}

impl Library {
    pub fn new(cfg: &Config) -> Self {
        let thumb_dir = expand_home(&cfg.thumb_dir);
        let dur_file = thumb_dir.join("durations.json");
        let dur_cache = std::fs::read(&dur_file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<HashMap<String, f64>>(&bytes).ok())
            .unwrap_or_default();
        Library {
            output_dir: expand_home(&cfg.output_dir),
            video_dir: expand_home(&cfg.video_dir),
            thumb_dir,
            dur_cache: Mutex::new(dur_cache),
            dur_file,
        }
    }

    /// 更新目录并保留视频时长缓存
    pub fn reconfigure(&mut self, cfg: &Config) {
        let out = expand_home(&cfg.output_dir);
        let vid = expand_home(&cfg.video_dir);
        let thumb = expand_home(&cfg.thumb_dir);
        if out != self.output_dir || vid != self.video_dir {
            self.dur_cache.lock().unwrap().clear();
        }
        self.output_dir = out;
        self.video_dir = vid;
        self.thumb_dir = thumb;
    }

    fn scan(&self) -> Vec<Raw> {
        let mut items = Vec::new();
        scan_dir(
            &self.output_dir,
            "image",
            &["png", "jpg", "jpeg", "webp"],
            &mut items,
        );
        scan_dir(
            &self.video_dir,
            "video",
            &["mp4", "mkv", "webm", "mov"],
            &mut items,
        );
        items.sort_by(|left, bytes| {
            bytes
                .mtime
                .partial_cmp(&left.mtime)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        items
    }

    fn duration_cached(&self, entry: &Raw) -> f64 {
        let key = format!("{}|{}", entry.path.to_string_lossy(), entry.mtime);
        {
            let cache = self.dur_cache.lock().unwrap();
            if let Some(duration_secs) = cache.get(&key) {
                return *duration_secs;
            }
        }
        let duration_secs = duration_of(&entry.path).unwrap_or(0.0);
        let snapshot = {
            let mut cache = self.dur_cache.lock().unwrap();
            cache.insert(key, duration_secs);
            serde_json::to_vec(&*cache).ok()
        };
        if let Some(body) = snapshot {
            let _ = std::fs::create_dir_all(&self.thumb_dir);
            let tmp = self.dur_file.with_extension("json.tmp");
            if std::fs::write(&tmp, body).is_ok() {
                let _ = std::fs::rename(&tmp, &self.dur_file);
            }
        }
        duration_secs
    }

    /// 轻量统计：图片数、视频数、总字节（不调用 ffprobe）
    pub fn stats(&self) -> (usize, usize, u64) {
        let raw = self.scan();
        let image_count = raw.iter().filter(|entry| entry.kind == "image").count();
        let video_count = raw.len() - image_count;
        let total_bytes = raw.iter().map(|entry| entry.size).sum();
        (image_count, video_count, total_bytes)
    }

    pub fn items(&self) -> Vec<Item> {
        self.scan()
            .into_iter()
            .map(|entry| {
                let duration = if entry.kind == "video" {
                    Some(self.duration_cached(&entry))
                } else {
                    None
                };
                Item {
                    id: entry.id,
                    kind: entry.kind,
                    name: entry.name,
                    size: entry.size,
                    mtime: entry.mtime,
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
        let path = dir.join(name);
        if path.is_file() {
            Some((kind.to_string(), name.to_string(), path))
        } else {
            None
        }
    }

    fn thumb_key(&self, it: &Item) -> u64 {
        let mut hasher = DefaultHasher::new();
        it.id.hash(&mut hasher);
        it.mtime.to_bits().hash(&mut hasher);
        it.size.hash(&mut hasher);
        hasher.finish()
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
        thumb.save_with_format(&dst, image::ImageFormat::Png).ok()?;
        Some(dst)
    }

    pub fn delete(&self, ids: &[String]) -> usize {
        let mut count = 0;
        for id in ids {
            if let Some((_, _, path)) = self.resolve(id) {
                if fs::remove_file(&path).is_ok() {
                    count += 1;
                }
            }
        }
        count
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
        if let Ok(text) = st {
            if text.success() && dst.is_file() {
                return true;
            }
        }
    }
    false
}

fn duration_of(path: &Path) -> Option<f64> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=nw=1:nk=1",
        ])
        .arg(path)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.trim().parse::<f64>().ok()
}
