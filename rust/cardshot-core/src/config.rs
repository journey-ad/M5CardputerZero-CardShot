use crate::util::expand_home;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct RecordCfg {
    pub fps: u32,
    pub bitrate: String,
    pub encoder: String,
    pub audio: bool,
}

impl Default for RecordCfg {
    fn default() -> Self {
        RecordCfg {
            fps: 20,
            bitrate: "2M".into(),
            encoder: "h264_v4l2m2m".into(),
            audio: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct HotkeysCfg {
    pub enabled: bool,
    pub shot: String,
    pub record: String,
    /// 独占物理键盘并转发其它按键，避免快捷键同时传给前台应用
    pub grab: bool,
}

impl Default for HotkeysCfg {
    fn default() -> Self {
        HotkeysCfg {
            enabled: true,
            shot: "shift+space".into(),
            record: "shift+enter".into(),
            grab: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct FeedbackCfg {
    pub led: bool,
}

impl Default for FeedbackCfg {
    fn default() -> Self {
        FeedbackCfg { led: true }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct RemoteCfg {
    pub enabled: bool,
    pub bind: String,
    pub port: u16,
}

impl Default for RemoteCfg {
    fn default() -> Self {
        RemoteCfg {
            enabled: true,
            bind: "0.0.0.0".into(),
            port: 8420,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub output_dir: String,
    pub video_dir: String,
    pub thumb_dir: String,
    pub image_format: String,
    pub image_quality: u8,
    pub socket_name: String,
    pub record: RecordCfg,
    pub hotkeys: HotkeysCfg,
    pub feedback: FeedbackCfg,
    pub remote: RemoteCfg,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            output_dir: "~/Pictures/CardShot".into(),
            video_dir: "~/Videos/CardShot".into(),
            thumb_dir: "~/.cache/cardshot/thumbs".into(),
            image_format: "png".into(),
            image_quality: 90,
            socket_name: "cardshot.sock".into(),
            record: RecordCfg::default(),
            hotkeys: HotkeysCfg::default(),
            feedback: FeedbackCfg::default(),
            remote: RemoteCfg::default(),
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Config {
        match std::fs::read_to_string(expand_home(path)) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let p = expand_home(path);
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let s = serde_json::to_string_pretty(self).unwrap_or_default();
        std::fs::write(p, s)
    }

    pub fn abs_output(&self) -> PathBuf {
        expand_home(&self.output_dir)
    }
    pub fn abs_video(&self) -> PathBuf {
        expand_home(&self.video_dir)
    }
    pub fn abs_thumb(&self) -> PathBuf {
        expand_home(&self.thumb_dir)
    }
}
