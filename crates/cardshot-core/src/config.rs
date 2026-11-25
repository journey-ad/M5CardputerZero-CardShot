use crate::util::expand_home;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ServiceCfg {
    /// 开机自启后台服务
    pub autostart: bool,
}

impl Default for ServiceCfg {
    fn default() -> Self {
        ServiceCfg { autostart: true }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct RecordCfg {
    pub fps: u32,
    pub bitrate: String,
    pub encoder: String,
    pub audio: bool,
    pub audio_device: String,
    pub max_seconds: u64,
}

impl Default for RecordCfg {
    fn default() -> Self {
        RecordCfg {
            fps: 20,
            bitrate: "2M".into(),
            encoder: "libx264".into(),
            audio: false,
            audio_device: "@DEFAULT_MONITOR@".to_string(),
            max_seconds: 120,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct HotkeysCfg {
    pub enabled: bool,
    pub shot: String,
    pub record: String,
}

impl Default for HotkeysCfg {
    fn default() -> Self {
        HotkeysCfg {
            enabled: true,
            shot: "shift+space".into(),
            record: "shift+enter".into(),
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
            port: 17173,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub output_dir: String,
    pub video_dir: String,
    pub thumb_dir: String,
    pub language: String,
    pub image_format: String,
    pub image_quality: u8,
    pub socket_name: String,
    pub service: ServiceCfg,
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
            language: "en".into(),
            image_format: "png".into(),
            image_quality: 90,
            socket_name: "cardshot.sock".into(),
            service: ServiceCfg::default(),
            record: RecordCfg::default(),
            hotkeys: HotkeysCfg::default(),
            feedback: FeedbackCfg::default(),
            remote: RemoteCfg::default(),
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Config {
        let resolved_path = expand_home(path);
        match std::fs::read_to_string(&resolved_path) {
            Ok(text) => serde_json::from_str(&text).expect("配置文件解析失败"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let cfg = Config::default();
                cfg.save(path).expect("写入默认配置失败");
                cfg
            }
            Err(error) => panic!("读取配置失败: {error}"),
        }
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let resolved_path = expand_home(path);
        if let Some(dir) = resolved_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).unwrap_or_default();
        std::fs::write(resolved_path, text)
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
