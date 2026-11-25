// 按键分发：按当前页面处理导航与操作

use serde_json::json;

use crate::app::actions::{apply_config, clear_all, clear_selected, delete_current, toggle_remote};
use crate::app::poll::{load_items, poll_config, poll_status};
use crate::app::view::open_preview;
use crate::constants::{
    FPS_STEPS, HomeAction, IMAGE_FORMATS, Page, SettingRow, GRID_COLS, HOME_COLS, PORT_MAX,
    PORT_MIN, QUALITY_PRESETS,
};
use crate::daemon::Client;
use crate::model::State;
use crate::AppWindow;

pub(crate) mod home;
pub(crate) mod gallery;
pub(crate) mod preview;
pub(crate) mod settings;

use home::handle_home;
use gallery::handle_gallery;
use preview::handle_preview;
use settings::{handle_settings, handle_hotkey_capture, handle_port_edit};

pub fn handle_key(ui: &AppWindow, st: &mut State, client: &mut Client, key: &str) {
    // Shift 状态单独跟踪，供快捷键录制与退格组合判断
    match key {
        "SHIFT_ON" => {
            st.shift_down = true;
            return;
        }
        "SHIFT_OFF" => {
            st.shift_down = false;
            return;
        }
        _ => {}
    }
    // 设置页的编辑态优先拦截按键
    if st.page == Page::Settings {
        if st.hotkey_capture {
            handle_hotkey_capture(st, client, key);
            return;
        }
        if st.port_edit {
            handle_port_edit(st, client, key);
            return;
        }
    }
    // 设置项说明面板：H / Esc / 空格 / 回车 关闭
    if st.page == Page::Settings && st.setting_help_visible {
        if matches!(key, "h" | "H" | "ESC" | "SPACE" | "ENTER") {
            st.setting_help_visible = false;
        }
        return;
    }
    // 设置页按 H 打开当前项说明
    if st.page == Page::Settings && matches!(key, "h" | "H") {
        st.setting_help_visible = true;
        return;
    }
    // 机身方向键为字母键，按设备按键布局映射为方向语义
    let key = match key {
        "z" | "Z" => "LEFT",
        "c" | "C" => "RIGHT",
        "f" | "F" => "UP",
        "x" | "X" | "s" | "S" => "DOWN",
        "w" | "W" => "UP",
        _ => key,
    };
    // 使用说明弹窗：任意确认键关闭
    if st.help_visible {
        if matches!(key, "ESC" | "ENTER" | "SPACE" | "BACK") {
            st.help_visible = false;
        }
        return;
    }
    // 删除二次确认弹窗：左右切换，Enter 执行，Esc 取消
    if st.confirm_visible {
        match key {
            "LEFT" | "RIGHT" => st.confirm_focus = 1 - st.confirm_focus,
            "ENTER" => {
                let do_it = st.confirm_focus == 1;
                let kind = st.confirm_kind;
                st.confirm_visible = false;
                st.confirm_kind = crate::constants::CONFIRM_DELETE;
                if do_it {
                    if kind == 1 {
                        clear_all(client, st);
                    } else {
                        delete_current(client, st);
                        if st.page == Page::Preview {
                            st.page = Page::Gallery;
                        }
                    }
                }
            }
            "ESC" => {
                st.confirm_visible = false;
                st.confirm_kind = crate::constants::CONFIRM_DELETE;
            }
            _ => {}
        }
        return;
    }
    // 全局按键
    match key {
        "TAB" if st.page == Page::Settings => {
            handle_settings(st, client, key);
            return;
        }
        "SHIFT_TAB" if st.page == Page::Settings => {
            handle_settings(st, client, key);
            return;
        }
        "SHIFT_TAB" => {
            st.page = st.page.prev();
            if st.page == Page::Gallery || st.page == Page::Preview {
                load_items(client, st);
            }
            if st.page == Page::Settings {
                poll_config(client, st);
            }
            return;
        }
        "TAB" => {
            st.page = st.page.next();
            if st.page == Page::Gallery || st.page == Page::Preview {
                load_items(client, st);
            }
            if st.page == Page::Settings {
                poll_config(client, st);
            }
            return;
        }
        "ESC" => {
            if st.page == Page::Gallery && st.selecting {
                st.selecting = false;
                st.selected.clear();
                return;
            }
            st.page = st.page.back();
            st.selecting = false;
            return;
        }
        _ => {}
    }

    match st.page {
        Page::Home => handle_home(ui, st, client, key),
        Page::Gallery => handle_gallery(ui, st, client, key),
        Page::Preview => handle_preview(ui, st, client, key),
        Page::Settings => handle_settings(st, client, key),
    }
    st.fix_scroll();
}
