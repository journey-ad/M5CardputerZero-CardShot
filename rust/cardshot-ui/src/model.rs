// 数据模型与界面状态

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use serde::Deserialize;
use slint::Image;

use crate::constants::{Page, DEFAULT_FPS, DEFAULT_PORT, DEFAULT_QUALITY};

#[derive(Deserialize, Clone)]
pub struct Item {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub size: u64,
    #[serde(default)]
    pub duration: Option<f64>,
}

pub fn human_size(n: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let n = n as f64;
    if n >= MB {
        format!("{:.1}MB", n / MB)
    } else if n >= KB {
        format!("{:.0}KB", n / KB)
    } else {
        format!("{}B", n as u64)
    }
}

pub fn fmt_duration(seconds: f64) -> String {
    let total = seconds.round() as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

pub struct State {
    pub page: Page,
    pub home_focus: usize,
    pub grid_focus: usize,
    pub scroll: usize,
    pub selecting: bool,
    pub selected: HashSet<usize>,
    pub settings_focus: usize,
    pub items: Vec<Item>,
    pub images: HashMap<String, Image>,
    pub images_grid: HashMap<String, Image>,
    pub count_images: usize,
    pub count_videos: usize,
    pub total_bytes: u64,
    pub recording: bool,
    pub rec_seconds: u64,
    pub image_format: String,
    pub image_quality: i64,
    pub fps: i64,
    pub remote_running: bool,
    pub remote_port: i64,
    pub hotkey_enabled: bool,
    pub shot_hotkey: String,
    pub output_dir: String,
    pub toast: String,
    pub toast_until: Option<Instant>,
    pub help_visible: bool,
    pub confirm_visible: bool,
    pub confirm_focus: i32,
    pub preview_name: String,
    pub preview_size: String,
    pub preview_dur: String,
    pub preview_kind: String,
    pub preview_pos: String,
    pub port_edit: bool,
    pub port_buf: String,
    pub hotkey_capture: bool,
    pub shift_down: bool,
}

impl State {
    pub fn new() -> State {
        State {
            page: Page::Home,
            home_focus: 0,
            grid_focus: 0,
            scroll: 0,
            selecting: false,
            selected: HashSet::new(),
            settings_focus: 0,
            items: Vec::new(),
            images: HashMap::new(),
            images_grid: HashMap::new(),
            count_images: 0,
            count_videos: 0,
            total_bytes: 0,
            recording: false,
            rec_seconds: 0,
            image_format: "png".into(),
            image_quality: DEFAULT_QUALITY,
            fps: DEFAULT_FPS,
            remote_running: true,
            remote_port: DEFAULT_PORT,
            hotkey_enabled: false,
            shot_hotkey: "shift+space".into(),
            output_dir: String::new(),
            toast: String::new(),
            toast_until: None,
            help_visible: false,
            confirm_visible: false,
            confirm_focus: 0,
            preview_name: String::new(),
            preview_size: String::new(),
            preview_dur: String::new(),
            preview_kind: "image".into(),
            preview_pos: String::new(),
            port_edit: false,
            port_buf: String::new(),
            hotkey_capture: false,
            shift_down: false,
        }
    }

    pub fn toast(&mut self, msg: &str) {
        self.toast = msg.to_string();
        self.toast_until = Some(Instant::now() + std::time::Duration::from_millis(crate::constants::TOAST_DURATION_MS));
    }

    pub fn toast_text(&self) -> String {
        match self.toast_until {
            Some(t) if Instant::now() < t => self.toast.clone(),
            _ => String::new(),
        }
    }

    pub fn total_items(&self) -> usize {
        self.count_images + self.count_videos
    }

    /// 保证当前选中项在当前页范围内
    pub fn fix_scroll(&mut self) {
        use crate::constants::GRID_PAGE;
        if self.items.is_empty() {
            self.scroll = 0;
            self.grid_focus = 0;
            return;
        }
        if self.grid_focus < self.scroll {
            self.scroll = self.grid_focus;
        }
        if self.grid_focus >= self.scroll + GRID_PAGE {
            self.scroll = self.grid_focus + 1 - GRID_PAGE;
        }
    }
}
