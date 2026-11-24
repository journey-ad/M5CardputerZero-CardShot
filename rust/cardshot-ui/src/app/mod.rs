// 应用控制器：组合界面、状态与守护客户端

pub mod actions;
pub mod keys;
pub mod poll;
pub mod view;

use crate::constants::{Page, SettingRow};
use slint::ComponentHandle;
use crate::daemon::Client;
use crate::model::State;
use crate::AppWindow;

pub struct App {
    ui: AppWindow,
    st: State,
    client: Option<Client>,
    quit: bool,
    esc_down: Option<std::time::Instant>,
}

impl App {
    pub fn new(ui: AppWindow) -> App {
        ui.global::<crate::I18n>().on_lookup(|text| crate::i18n::tr_current(text.as_str()).into());
        App {
            ui,
            st: State::new(),
            client: Client::connect(),
            quit: false,
            esc_down: None,
        }
    }

    /// 初始化：拉取一次状态与配置，并支持通过环境变量指定起始页（便于调试）
    pub fn init(&mut self) {
        let App { ui, st, client, .. } = self;
        let Some(c) = client.as_mut() else { return };
        poll::poll_status(c, st);
        poll::poll_config(c, st);
        if let Ok(page) = std::env::var("CARDSHOT_START_PAGE").unwrap_or_default().parse::<i32>() {
            st.page = Page::ALL
                .iter()
                .copied()
                .find(|p| p.index() == page)
                .unwrap_or(Page::Home);
        }
        if st.page != Page::Home {
            poll::load_items(c, st);
        }
        if let Ok(idx) = std::env::var("CARDSHOT_TEST_INDEX").unwrap_or_default().parse::<usize>() {
            st.grid_focus = idx;
        }
        // 起始页为预览时，等列表加载完成后由 tick 打开首项
        if st.page == Page::Preview {
            st.pending_preview = true;
        }
        // 调试钩子：直接展示弹窗，便于离线截图验证
        if let Ok(l) = std::env::var("CARDSHOT_TEST_LANG") {
            st.language = crate::i18n::normalize(&l).to_string();
        }
        if let Ok(idx) = std::env::var("CARDSHOT_TEST_SETTINGS").unwrap_or_default().parse::<usize>() {
            st.settings_focus = idx;
            st.settings_tab = crate::constants::settings_tab_of(idx);
        }
        if std::env::var("CARDSHOT_TEST_HELP").is_ok() {
            st.help_visible = true;
        }
        if std::env::var("CARDSHOT_TEST_CONFIRM").is_ok() {
            st.confirm_visible = true;
            st.confirm_focus = 1;
        }
        if std::env::var("CARDSHOT_TEST_CLEAR").is_ok() {
            st.settings_focus = SettingRow::Clear.index();
            st.settings_tab = 3;
            st.confirm_kind = 1;
            st.confirm_visible = true;
            st.confirm_focus = 1;
        }
        if std::env::var("CARDSHOT_TEST_SELECT").is_ok() {
            st.selecting = true;
            st.selected.insert(st.grid_focus);
        }
        if std::env::var("CARDSHOT_TEST_ENDED").is_ok() {
            st.video_ended = true;
        }
        if std::env::var("CARDSHOT_TEST_PORT_EDIT").is_ok() {
            st.settings_focus = SettingRow::Port.index();
            st.settings_tab = 2;
            st.port_edit = true;
            st.port_buf = "8420".into();
        }
        if std::env::var("CARDSHOT_TEST_HOTKEY").is_ok() {
            st.settings_focus = SettingRow::ShotHotkey.index();
            st.settings_tab = 1;
            st.hotkey_capture = true;
        }
    }

    pub fn on_key(&mut self, key: &str) {
        // 主界面长按 ESC 退出；其余场景 ESC 均为返回
        let modal = self.st.help_visible
            || self.st.confirm_visible
            || self.st.port_edit
            || self.st.hotkey_capture;
        if key == "ESC" {
            if self.st.page == Page::Home && !modal {
                self.esc_down = Some(std::time::Instant::now());
                return;
            }
        } else if key == "ESC_UP" {
            if let Some(t) = self.esc_down.take() {
                if self.st.page == Page::Home && t.elapsed() >= std::time::Duration::from_millis(700) {
                    self.quit = true;
                }
            }
            return;
        }
        if self.esc_down.is_some() {
            self.esc_down = None;
        }
        let App { ui, st, client, .. } = self;
        if let Some(c) = client.as_mut() {
            keys::handle_key(ui, st, c, key);
        }
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn poll(&mut self) {
        let App { st, client, .. } = self;
        if let Some(c) = client.as_mut() {
            poll::poll_status(c, st);
        }
    }

    /// 相册与预览页在数据变化时重建列表模型
    pub fn rebuild_model(&mut self) {
        let App { ui, st, client, .. } = self;
        if matches!(st.page, Page::Gallery | Page::Preview) {
            if let Some(c) = client.as_mut() {
                view::set_gallery_model(ui, c, st);
            }
        }
    }

    pub fn sync(&mut self) {
        view::sync_ui(&self.ui, &self.st);
    }

    /// 每帧推进：离开预览页时停止视频，预览页刷新播放帧
    pub fn tick(&mut self) {
        let App { ui, st, client, .. } = self;
        if st.pending_preview && !st.items.is_empty() {
            if let Some(c) = client.as_mut() {
                view::open_preview(ui, c, st);
            }
            st.pending_preview = false;
        }
        if !matches!(st.page, Page::Preview) {
            view::stop_video(st);
        }
        view::tick_video(ui, st);
    }

    pub fn wants_model(&self) -> bool {
        matches!(self.st.page, Page::Gallery | Page::Preview)
    }
}
