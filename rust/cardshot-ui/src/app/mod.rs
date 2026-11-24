// 应用控制器：组合界面、状态与守护客户端

pub mod actions;
pub mod keys;
pub mod poll;
pub mod view;

use crate::constants::Page;
use crate::daemon::Client;
use crate::model::State;
use crate::AppWindow;

pub struct App {
    ui: AppWindow,
    st: State,
    client: Option<Client>,
    quit: bool,
}

impl App {
    pub fn new(ui: AppWindow) -> App {
        App {
            ui,
            st: State::new(),
            client: Client::connect(),
            quit: false,
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
        if st.page == Page::Preview {
            view::open_preview(ui, c, st);
        }
        // 调试钩子：直接展示弹窗，便于离线截图验证
        if std::env::var("CARDSHOT_TEST_HELP").is_ok() {
            st.help_visible = true;
        }
        if std::env::var("CARDSHOT_TEST_CONFIRM").is_ok() {
            st.confirm_visible = true;
            st.confirm_focus = 1;
        }
    }

    pub fn on_key(&mut self, key: &str) {
        // 主页按 Esc 退出，其余页面由按键处理逐级返回
        if key == "ESC" && self.st.page == Page::Home {
            self.quit = true;
            return;
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

    pub fn wants_model(&self) -> bool {
        matches!(self.st.page, Page::Gallery | Page::Preview)
    }
}
