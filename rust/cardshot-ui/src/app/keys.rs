// 按键分发：按当前页面处理导航与操作

use serde_json::json;

use crate::app::actions::{apply_config, delete_current, toggle_remote};
use crate::app::poll::{load_items, poll_config, poll_status};
use crate::app::view::open_preview;
use crate::constants::{
    FPS_STEPS, HomeAction, IMAGE_FORMATS, Page, SettingRow, GRID_COLS, HOME_COLS, PORT_MAX,
    PORT_MIN, PORT_STEP, QUALITY_MAX, QUALITY_MIN, QUALITY_STEP, SHOT_CHORDS,
};
use crate::daemon::Client;
use crate::model::State;
use crate::AppWindow;

pub fn handle_key(ui: &AppWindow, st: &mut State, client: &mut Client, key: &str) {
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
                let do_delete = st.confirm_focus == 1;
                st.confirm_visible = false;
                if do_delete {
                    delete_current(client, st);
                    if st.page == Page::Preview {
                        st.page = Page::Gallery;
                    }
                }
            }
            "ESC" => st.confirm_visible = false,
            _ => {}
        }
        return;
    }
    // 全局按键
    match key {
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

fn handle_home(ui: &AppWindow, st: &mut State, client: &mut Client, key: &str) {
    let count = HomeAction::LIST.len();
    match key {
        "LEFT" => {
            if st.home_focus % HOME_COLS > 0 {
                st.home_focus -= 1;
            }
        }
        "RIGHT" => {
            if st.home_focus % HOME_COLS + 1 < HOME_COLS && st.home_focus + 1 < count {
                st.home_focus += 1;
            }
        }
        "UP" => {
            if st.home_focus >= HOME_COLS {
                st.home_focus -= HOME_COLS;
            }
        }
        "DOWN" => {
            if st.home_focus + HOME_COLS < count {
                st.home_focus += HOME_COLS;
            }
        }
        "ENTER" => match HomeAction::at(st.home_focus) {
            HomeAction::Gallery => {
                load_items(client, st);
                st.page = Page::Gallery;
            }
            HomeAction::Settings => {
                poll_config(client, st);
                st.page = Page::Settings;
            }
            HomeAction::Help => st.help_visible = true,
        },
        _ => {}
    }
    let _ = ui;
}

fn handle_gallery(ui: &AppWindow, st: &mut State, client: &mut Client, key: &str) {
    let len = st.items.len();
    match key {
        "LEFT" => {
            if st.grid_focus > 0 {
                st.grid_focus -= 1;
            }
        }
        "RIGHT" => {
            if st.grid_focus + 1 < len {
                st.grid_focus += 1;
            }
        }
        "UP" => {
            if st.grid_focus >= GRID_COLS {
                st.grid_focus -= GRID_COLS;
            }
        }
        "DOWN" => {
            if st.grid_focus + GRID_COLS < len {
                st.grid_focus += GRID_COLS;
            }
        }
        "SPACE" => {
            st.selecting = true;
            if st.selected.contains(&st.grid_focus) {
                st.selected.remove(&st.grid_focus);
            } else {
                st.selected.insert(st.grid_focus);
            }
        }
        "ENTER" => open_preview(ui, client, st),
        "BACK" => {
            st.confirm_focus = 0;
            st.confirm_visible = true;
        }
        "SHIFT_BACK" => delete_current(client, st),
        _ => {}
    }
}

fn handle_preview(ui: &AppWindow, st: &mut State, client: &mut Client, key: &str) {
    match key {
        "LEFT" => {
            if st.grid_focus > 0 {
                st.grid_focus -= 1;
                open_preview(ui, client, st);
            }
        }
        "RIGHT" => {
            if st.grid_focus + 1 < st.items.len() {
                st.grid_focus += 1;
                open_preview(ui, client, st);
            }
        }
        "BACK" => {
            st.confirm_focus = 0;
            st.confirm_visible = true;
        }
        "SHIFT_BACK" => {
            delete_current(client, st);
            st.page = Page::Gallery;
        }
        _ => {}
    }
}

/// 修改端口：先写配置，再按需重启远程服务使端口立即生效
fn change_port(client: &mut Client, st: &mut State, port: i64) {
    apply_config(client, st, json!({ "remote": { "port": port } }));
    if st.remote_running {
        client.set_remote(false);
        client.set_remote(true);
        poll_status(client, st);
        poll_config(client, st);
    }
}

fn handle_settings(st: &mut State, client: &mut Client, key: &str) {
    let row_count = SettingRow::LIST.len();
    match key {
        "UP" => {
            if st.settings_focus > 0 {
                st.settings_focus -= 1;
            }
        }
        "DOWN" => {
            if st.settings_focus + 1 < row_count {
                st.settings_focus += 1;
            }
        }
        "LEFT" | "RIGHT" => {
            let dir: i64 = if key == "RIGHT" { 1 } else { -1 };
            match SettingRow::at(st.settings_focus) {
                SettingRow::Format => {
                    let current = IMAGE_FORMATS
                        .iter()
                        .position(|f| *f == st.image_format)
                        .unwrap_or(0) as i64;
                    let next = (current + dir).rem_euclid(IMAGE_FORMATS.len() as i64) as usize;
                    apply_config(client, st, json!({ "image_format": IMAGE_FORMATS[next] }));
                }
                SettingRow::Quality => {
                    let q = (st.image_quality + dir * QUALITY_STEP).clamp(QUALITY_MIN, QUALITY_MAX);
                    apply_config(client, st, json!({ "image_quality": q }));
                }
                SettingRow::Fps => {
                    let current = FPS_STEPS.iter().position(|v| *v == st.fps).unwrap_or(0) as i64;
                    let next = (current + dir).rem_euclid(FPS_STEPS.len() as i64) as usize;
                    apply_config(client, st, json!({ "record": { "fps": FPS_STEPS[next] } }));
                }
                SettingRow::Port => {
                    let port = (st.remote_port + dir * PORT_STEP).clamp(PORT_MIN, PORT_MAX);
                    if port != st.remote_port {
                        change_port(client, st, port);
                    }
                }
                SettingRow::ShotHotkey => {
                    let current = SHOT_CHORDS
                        .iter()
                        .position(|v| *v == st.shot_hotkey)
                        .unwrap_or(0) as i64;
                    let next = (current + dir).rem_euclid(SHOT_CHORDS.len() as i64) as usize;
                    apply_config(client, st, json!({ "hotkeys": { "shot": SHOT_CHORDS[next] } }));
                }
                SettingRow::Remote => toggle_remote(client, st),
            }
        }
        _ => {}
    }
}
