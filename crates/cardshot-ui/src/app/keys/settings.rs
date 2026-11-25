use super::*;

pub(crate) fn change_port(client: &mut Client, st: &mut State, port: i64) {
    apply_config(client, st, json!({ "remote": { "port": port } }));
    if st.remote_running {
        client.set_remote(false);
        client.set_remote(true);
        poll_status(client, st);
        poll_config(client, st);
    }
}

pub(crate) fn hotkey_mods_upper(st: &State) -> Vec<String> {
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

pub(crate) fn hotkey_prefix(st: &State, upper: bool) -> String {
    let mods = hotkey_mods_upper(st);
    if mods.is_empty() {
        return String::new();
    }
    let joined = mods
        .iter()
        .map(|modifier| {
            if upper {
                modifier.clone()
            } else {
                modifier.to_lowercase()
            }
        })
        .collect::<Vec<_>>()
        .join("+");
    format!("{joined}+")
}

pub(crate) fn refresh_hotkey_buffer(st: &mut State) {
    st.hotkey_buffer = hotkey_prefix(st, false);
}

pub(crate) fn handle_hotkey_capture(st: &mut State, client: &mut Client, key: &str) {
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
            st.hotkey_record = false;
            st.hotkey_buffer.clear();
            st.shift_down = false;
            st.ctrl_down = false;
            st.alt_down = false;
        }
        "BACK" | "SHIFT_BACK" => {
            // 按住修饰键时退格不提交，继续等待组合键
            if !any_mod {
                st.hotkey_capture = false;
                st.hotkey_record = false;
                st.hotkey_buffer.clear();
            }
        }
        "UP" | "DOWN" | "LEFT" | "RIGHT" => {}
        _ => {
            let base = key.to_lowercase();
            let chord = format!("{}{}", hotkey_prefix(st, false), base);
            if st.hotkey_record {
                apply_config(client, st, json!({ "hotkeys": { "record": chord } }));
                st.record_hotkey = chord;
            } else {
                apply_config(client, st, json!({ "hotkeys": { "shot": chord } }));
                st.shot_hotkey = chord;
            }
            st.hotkey_capture = false;
            st.hotkey_buffer.clear();
            st.shift_down = false;
            st.ctrl_down = false;
            st.alt_down = false;
        }
    }
}

pub(crate) fn handle_port_edit(st: &mut State, client: &mut Client, key: &str) {
    match key {
        "ENTER" => {
            if let Ok(port_value) = st.port_buf.parse::<i64>() {
                let port_value = port_value.clamp(PORT_MIN, PORT_MAX);
                if port_value != st.remote_port {
                    change_port(client, st, port_value);
                }
            }
            st.port_edit = false;
        }
        "ESC" => st.port_edit = false,
        "BACK" => {
            st.port_buf.pop();
        }
        key if key.len() == 1 && key.chars().all(|character| character.is_ascii_digit()) => {
            if st.port_buf.len() < 5 {
                st.port_buf.push_str(key);
            }
        }
        _ => {}
    }
}

pub(crate) fn cycle<T: Copy + PartialEq>(list: &[T], current: T, dir: i64) -> T {
    let idx = list.iter().position(|item| *item == current).unwrap_or(0) as i64;
    let next = (idx + dir).rem_euclid(list.len() as i64) as usize;
    list[next]
}

pub(crate) fn handle_settings(st: &mut State, client: &mut Client, key: &str) {
    let (start, end) = crate::constants::SETTINGS_TAB_RANGES[st.settings_tab];
    match key {
        "TAB" => {
            st.settings_tab = (st.settings_tab + 1) % crate::constants::SETTINGS_TAB_RANGES.len();
            st.settings_focus = crate::constants::SETTINGS_TAB_RANGES[st.settings_tab].0;
        }
        "SHIFT_TAB" => {
            let tab_count = crate::constants::SETTINGS_TAB_RANGES.len();
            st.settings_tab = (st.settings_tab + tab_count - 1) % tab_count;
            st.settings_focus = crate::constants::SETTINGS_TAB_RANGES[st.settings_tab].0;
        }
        "UP" => {
            st.settings_focus = if st.settings_focus > start {
                st.settings_focus - 1
            } else {
                end
            };
        }
        "DOWN" => {
            st.settings_focus = if st.settings_focus < end {
                st.settings_focus + 1
            } else {
                start
            };
        }
        "LEFT" | "RIGHT" | "ENTER" => {
            let dir: i64 = if key == "LEFT" { -1 } else { 1 };
            match SettingRow::at(st.settings_focus) {
                SettingRow::Autostart => {
                    let next = !st.autostart;
                    apply_config(client, st, json!({ "service": { "autostart": next } }));
                    st.autostart = next;
                }
                SettingRow::Language => {
                    let cur = st.language.clone();
                    let next = cycle(&crate::i18n::LANGS, cur.as_str(), dir);
                    apply_config(client, st, json!({ "language": next }));
                    st.language = crate::i18n::normalize(next).to_string();
                }
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
                SettingRow::RecordAudio => {
                    let next = !st.record_audio;
                    apply_config(client, st, json!({ "record": { "audio": next } }));
                    st.record_audio = next;
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
                        st.hotkey_record = false;
                        st.hotkey_buffer.clear();
                        st.shift_down = false;
                        st.ctrl_down = false;
                        st.alt_down = false;
                    }
                }
                SettingRow::RecordHotkey => {
                    if key == "ENTER" {
                        st.hotkey_capture = true;
                        st.hotkey_record = true;
                        st.hotkey_buffer.clear();
                        st.shift_down = false;
                        st.ctrl_down = false;
                        st.alt_down = false;
                    }
                }
                SettingRow::Clear => {
                    if key == "ENTER" {
                        st.confirm_kind = crate::constants::CONFIRM_CLEAR;
                        st.confirm_focus = 0;
                        st.confirm_visible = true;
                    }
                }
            }
        }
        _ => {}
    }
}
