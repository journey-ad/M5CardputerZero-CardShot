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
/// 压缩质量预设档位（左右键循环）
pub const QUALITY_PRESETS: [i64; 6] = [60, 75, 85, 90, 95, 100];
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
pub const PREVIEW_BOX: (u32, u32) = (WIDTH, HEIGHT);

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
            Page::Settings => "",
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
    ShotHotkey,
    RecordHotkey,
    Remote,
    Port,
    Clear,
}

impl SettingRow {
    pub const LIST: [SettingRow; 8] = [
        SettingRow::Format,
        SettingRow::Quality,
        SettingRow::Fps,
        SettingRow::ShotHotkey,
        SettingRow::RecordHotkey,
        SettingRow::Remote,
        SettingRow::Port,
        SettingRow::Clear,
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
    pub const LEFT_CTRL: u16 = 29;
    pub const RIGHT_CTRL: u16 = 97;
    pub const LEFT_ALT: u16 = 56;
    pub const RIGHT_ALT: u16 = 100;
    pub const UP: u16 = 103;
    pub const DOWN: u16 = 108;
    pub const LEFT: u16 = 105;
    pub const RIGHT: u16 = 106;
    pub const KEY_A: u16 = 30;
    pub const KEY_B: u16 = 48;
    pub const KEY_C: u16 = 46;
    pub const KEY_D: u16 = 32;
    pub const KEY_E: u16 = 18;
    pub const KEY_F: u16 = 33;
    pub const KEY_G: u16 = 34;
    pub const KEY_H: u16 = 35;
    pub const KEY_I: u16 = 23;
    pub const KEY_J: u16 = 36;
    pub const KEY_K: u16 = 37;
    pub const KEY_L: u16 = 38;
    pub const KEY_M: u16 = 50;
    pub const KEY_N: u16 = 49;
    pub const KEY_O: u16 = 24;
    pub const KEY_P: u16 = 25;
    pub const KEY_Q: u16 = 16;
    pub const KEY_R: u16 = 19;
    pub const KEY_S: u16 = 31;
    pub const KEY_T: u16 = 20;
    pub const KEY_U: u16 = 22;
    pub const KEY_V: u16 = 47;
    pub const KEY_W: u16 = 17;
    pub const KEY_X: u16 = 45;
    pub const KEY_Y: u16 = 21;
    pub const KEY_Z: u16 = 44;
    pub const KEY_1: u16 = 2;
    pub const KEY_2: u16 = 3;
    pub const KEY_3: u16 = 4;
    pub const KEY_4: u16 = 5;
    pub const KEY_5: u16 = 6;
    pub const KEY_6: u16 = 7;
    pub const KEY_7: u16 = 8;
    pub const KEY_8: u16 = 9;
    pub const KEY_9: u16 = 10;
    pub const KEY_0: u16 = 11;
}
