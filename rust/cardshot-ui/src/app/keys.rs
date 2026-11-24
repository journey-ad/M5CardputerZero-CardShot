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
    // 机身方向键为字母键，按 cardbili 的约定映射为方向语义
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
                st.confirm_kind = 0;
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
                st.confirm_kind = 0;
            }
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
            if st.selecting {
                st.selecting = false;
                st.selected.clear();
            } else {
                st.confirm_kind = 0;
                st.confirm_focus = 0;
                st.confirm_visible = true;
            }
        }
        "SHIFT_BACK" => {
            if st.selecting && st.selected.len() > 1 {
                clear_selected(client, st);
            } else {
                delete_current(client, st);
            }
        }
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
        "ENTER" | "SPACE" => {
            if st.video_ended && st.preview_kind == "video" {
                st.video_ended = false;
                open_preview(ui, client, st);
            }
        }
        "BACK" => {
            st.confirm_kind = 0;
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

fn hotkey_mods_upper(st: &State) -> Vec<String> {
    let mut mods = Vec::new();
    if st.ctrl_down {
        mods.push("Ctrl".to_string());
    }
    if st.alt_down {
        mods.push("Alt".to_string());
    }
    if st.shift_down {
        mods.push("Shift".to_string());
    }
    mods
}

/// 仅显示已按下的修饰键前缀，例如 "Ctrl+Alt+"；未按修饰键时为空
fn hotkey_prefix(st: &State, upper: bool) -> String {
    let mods = hotkey_mods_upper(st);
    if mods.is_empty() {
        return String::new();
    }
    let joined = mods.iter().map(|m| if upper { m.clone() } else { m.to_lowercase() }).collect::<Vec<_>>().join("+");
    format!("{joined}+")
}

fn refresh_hotkey_buffer(st: &mut State) {
    st.hotkey_buffer = hotkey_prefix(st, true);
}

fn handle_hotkey_capture(st: &mut State, client: &mut Client, key: &str) {
    match key {
        "SHIFT_ON" => {
            st.shift_down = true;
            refresh_hotkey_buffer(st);
            return;
        }
        "SHIFT_OFF" => {
            st.shift_down = false;
            refresh_hotkey_buffer(st);
            return;
        }
        "CTRL_ON" => {
            st.ctrl_down = true;
            refresh_hotkey_buffer(st);
            return;
        }
        "CTRL_OFF" => {
            st.ctrl_down = false;
            refresh_hotkey_buffer(st);
            return;
        }
        "ALT_ON" => {
            st.alt_down = true;
            refresh_hotkey_buffer(st);
            return;
        }
        "ALT_OFF" => {
            st.alt_down = false;
            refresh_hotkey_buffer(st);
            return;
        }
        _ => {}
    }
    let any_mod = st.ctrl_down || st.alt_down || st.shift_down;
    match key {
        "ESC" => {
            st.hotkey_capture = false;
            st.hotkey_buffer.clear();
            st.shift_down = false;
            st.ctrl_down = false;
            st.alt_down = false;
        }
        "BACK" | "SHIFT_BACK" => {
            // 按住修饰键时退格不提交，继续等待组合键
            if !any_mod {
                st.hotkey_capture = false;
                st.hotkey_buffer.clear();
            }
        }
        "UP" | "DOWN" | "LEFT" | "RIGHT" => {}
        _ => {
            let base = key.to_lowercase();
            let chord = format!("{}{}", hotkey_prefix(st, false), base);
            apply_config(client, st, json!({ "hotkeys": { "shot": chord } }));
            st.shot_hotkey = chord;
            st.hotkey_capture = false;
            st.hotkey_buffer.clear();
            st.shift_down = false;
            st.ctrl_down = false;
            st.alt_down = false;
        }
    }
}

fn handle_port_edit(st: &mut State, client: &mut Client, key: &str) {
    match key {
        "ENTER" => {
            if let Ok(p) = st.port_buf.parse::<i64>() {
                let p = p.clamp(PORT_MIN, PORT_MAX);
                if p != st.remote_port {
                    change_port(client, st, p);
                }
            }
            st.port_edit = false;
        }
        "ESC" => st.port_edit = false,
        "BACK" => {
            st.port_buf.pop();
        }
        k if k.len() == 1 && k.chars().all(|c| c.is_ascii_digit()) => {
            if st.port_buf.len() < 5 {
                st.port_buf.push_str(k);
            }
        }
        _ => {}
    }
}

fn cycle<T: Copy + PartialEq>(list: &[T], current: T, dir: i64) -> T {
    let idx = list.iter().position(|v| *v == current).unwrap_or(0) as i64;
    let next = (idx + dir).rem_euclid(list.len() as i64) as usize;
    list[next]
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
        "LEFT" | "RIGHT" | "ENTER" => {
            let dir: i64 = if key == "LEFT" { -1 } else { 1 };
            match SettingRow::at(st.settings_focus) {
                SettingRow::Format => {
                    let next = cycle(&IMAGE_FORMATS, st.image_format.as_str(), dir);
                    apply_config(client, st, json!({ "image_format": next }));
                }
                SettingRow::Quality => {
                    let next = cycle(&QUALITY_PRESETS, st.image_quality, dir);
                    apply_config(client, st, json!({ "image_quality": next }));
                }
                SettingRow::Fps => {
                    let next = cycle(&FPS_STEPS, st.fps, dir);
                    apply_config(client, st, json!({ "record": { "fps": next } }));
                }
                SettingRow::Remote => {
                    if matches!(key, "ENTER" | "LEFT" | "RIGHT") {
                        toggle_remote(client, st);
                    }
                }
                SettingRow::Port => {
                    if key == "ENTER" && st.remote_running {
                        st.port_edit = true;
                        st.port_buf = st.remote_port.to_string();
                    }
                }
                SettingRow::ShotHotkey => {
                    if key == "ENTER" {
                        st.hotkey_capture = true;
                        st.hotkey_buffer.clear();
                        st.shift_down = false;
                        st.ctrl_down = false;
                        st.alt_down = false;
                    }
                }
                SettingRow::Clear => {
                    if key == "ENTER" {
                        st.confirm_kind = 1;
                        st.confirm_focus = 0;
                        st.confirm_visible = true;
                    }
                }
            }
        }
        _ => {}
    }
}
