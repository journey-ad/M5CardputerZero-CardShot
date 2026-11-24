// 集中常量与语义化枚举，避免散落的魔法值与下标

pub const WIDTH: u32 = 320;
pub const HEIGHT: u32 = 170;
pub const FB_BYTES: usize = WIDTH as usize * HEIGHT as usize * 2;

/// 相册网格：每页列数与行数，页容量由两者相乘得到
pub const GRID_COLS: usize = 3;
pub const GRID_ROWS: usize = 2;
pub const GRID_PAGE: usize = GRID_COLS * GRID_ROWS;

/// 主页卡片按单列纵向排列
pub const HOME_COLS: usize = 1;

pub const FRAME_SLEEP_MS: u64 = 30;
pub const POLL_INTERVAL_MS: u64 = 500;
pub const TOAST_DURATION_MS: u64 = 2000;

pub const DEFAULT_PORT: i64 = 8420;
pub const DEFAULT_QUALITY: i64 = 90;
pub const DEFAULT_FPS: i64 = 20;

pub const QUALITY_MIN: i64 = 1;
pub const QUALITY_MAX: i64 = 100;
pub const QUALITY_STEP: i64 = 10;
pub const IMAGE_FORMATS: [&str; 3] = ["png", "jpeg", "webp"];
pub const FPS_STEPS: [i64; 5] = [10, 15, 20, 25, 30];

/// 端口取值范围与调节步进
pub const PORT_MIN: i64 = 1024;
pub const PORT_MAX: i64 = 65535;
pub const PORT_STEP: i64 = 10;

/// 截图快捷键候选项，左右键循环切换
pub const SHOT_CHORDS: [&str; 5] = [
    "shift+space",
    "ctrl+space",
    "shift+s",
    "ctrl+alt+s",
    "f12",
];

/// 缩略图与预览图的最大显示尺寸
pub const THUMB_BOX: (u32, u32) = (96, 40);
pub const PREVIEW_BOX: (u32, u32) = (WIDTH, 110);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Home,
    Gallery,
    Preview,
    Settings,
}

impl Page {
    pub const ALL: [Page; 4] = [Page::Home, Page::Gallery, Page::Preview, Page::Settings];

    pub fn index(self) -> i32 {
        match self {
            Page::Home => 0,
            Page::Gallery => 1,
            Page::Preview => 2,
            Page::Settings => 3,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Page::Home => "CardShot",
            Page::Gallery => "相册",
            Page::Preview => "预览",
            Page::Settings => "设置",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Page::Home => "Enter 打开",
            Page::Gallery => "Enter 预览   空格 多选   退格 删除",
            Page::Preview => "退格 删除   Esc 返回",
            Page::Settings => "▲▼ 选择   ◀▶ 调整",
        }
    }

    /// 切换到的下一页，按 Page::ALL 顺序循环
    pub fn next(self) -> Page {
        let list = Page::ALL;
        let pos = list.iter().position(|p| *p == self).unwrap_or(0);
        list[(pos + 1) % list.len()]
    }

    /// Esc 返回的目标页
    pub fn back(self) -> Page {
        match self {
            Page::Preview => Page::Gallery,
            _ => Page::Home,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HomeAction {
    Gallery,
    Settings,
    Help,
}

impl HomeAction {
    pub const LIST: [HomeAction; 3] = [HomeAction::Gallery, HomeAction::Settings, HomeAction::Help];

    pub fn at(index: usize) -> HomeAction {
        HomeAction::LIST[index.min(HomeAction::LIST.len() - 1)]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingRow {
    Format,
    Quality,
    Fps,
    Port,
    ShotHotkey,
    Remote,
}

impl SettingRow {
    pub const LIST: [SettingRow; 6] = [
        SettingRow::Format,
        SettingRow::Quality,
        SettingRow::Fps,
        SettingRow::Port,
        SettingRow::ShotHotkey,
        SettingRow::Remote,
    ];

    pub fn at(index: usize) -> SettingRow {
        SettingRow::LIST[index.min(SettingRow::LIST.len() - 1)]
    }
}

/// evdev 键码
pub mod keycode {
    pub const ESC: u16 = 1;
    pub const BACKSPACE: u16 = 14;
    pub const TAB: u16 = 15;
    pub const ENTER: u16 = 28;
    pub const SPACE: u16 = 57;
    pub const SHIFT_LEFT: u16 = 42;
    pub const SHIFT_RIGHT: u16 = 54;
    pub const UP: u16 = 103;
    pub const DOWN: u16 = 108;
    pub const LEFT: u16 = 105;
    pub const RIGHT: u16 = 106;
    pub const KEY_D: u16 = 32;
    pub const KEY_M: u16 = 50;
}
