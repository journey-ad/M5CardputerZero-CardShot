pub(crate) fn setting_tabs(st: &crate::model::State) -> Vec<slint::SharedString> {
    let lang = st.language.as_str();
    ["常规", "按键", "远程", "数据"]
        .iter()
        .map(|key| crate::i18n::tr(lang, key).into())
        .collect()
}

pub(crate) fn hotkey_text(st: &crate::model::State, record: bool) -> String {
    let editing = if record {
        st.hotkey_record
    } else {
        st.hotkey_capture
    };
    if editing {
        if st.hotkey_buffer.is_empty() {
            "按下组合键…".to_string()
        } else {
            st.hotkey_buffer.clone()
        }
    } else if record {
        st.record_hotkey.clone()
    } else {
        st.shot_hotkey.clone()
    }
}

pub(crate) fn setting_rows(st: &crate::model::State) -> Vec<crate::SettingItem> {
    use crate::constants::SettingRow;
    let lang = st.language.as_str();
    let translate = |key: &str| crate::i18n::tr(lang, key);
    let on_off = |flag: bool| translate(if flag { "开" } else { "关" }).to_string();
    let row = |row: SettingRow,
               label: String,
               mode: i32,
               value: String,
               value2: String,
               disabled: bool| {
        crate::SettingItem {
            label: label.into(),
            mode,
            value: value.into(),
            value2: value2.into(),
            focused: st.settings_focus == row.index(),
            disabled,
        }
    };
    let language_label = translate("语言设置").to_string();
    let port_value = if st.port_edit {
        format!("{}_", st.port_buf)
    } else {
        st.remote_port.to_string()
    };

    match st.settings_tab {
        0 => vec![
            row(
                SettingRow::Autostart,
                translate("后台常驻"),
                2,
                on_off(st.autostart),
                "".into(),
                false,
            ),
            row(
                SettingRow::Language,
                language_label,
                0,
                crate::i18n::lang_name(lang).to_string(),
                "".into(),
                false,
            ),
            row(
                SettingRow::Format,
                translate("图片格式"),
                1,
                st.image_format.to_uppercase(),
                "".into(),
                false,
            ),
            row(
                SettingRow::Quality,
                translate("压缩质量"),
                0,
                st.image_quality.to_string(),
                "".into(),
                false,
            ),
            row(
                SettingRow::Fps,
                translate("录制帧率"),
                0,
                st.fps.to_string(),
                "".into(),
                false,
            ),
        ],
        1 => vec![
            row(
                SettingRow::ShotHotkey,
                translate("截图快捷键"),
                0,
                hotkey_text(st, false),
                "".into(),
                false,
            ),
            row(
                SettingRow::RecordHotkey,
                translate("录制快捷键"),
                0,
                hotkey_text(st, true),
                "".into(),
                false,
            ),
            row(
                SettingRow::RecordAudio,
                translate("录制声音"),
                2,
                on_off(st.record_audio),
                "".into(),
                false,
            ),
        ],
        2 => vec![
            row(
                SettingRow::Remote,
                translate("Web管理面板"),
                2,
                on_off(st.remote_running),
                "".into(),
                false,
            ),
            row(
                SettingRow::Port,
                translate("服务端口"),
                3,
                if st.remote_host.is_empty() {
                    String::new()
                } else {
                    format!("{}:", st.remote_host)
                },
                port_value,
                !st.remote_running,
            ),
        ],
        _ => vec![row(
            SettingRow::Clear,
            translate("清空数据"),
            0,
            translate("全部删除"),
            "".into(),
            false,
        )],
    }
}

pub(crate) fn setting_footer(st: &crate::model::State) -> String {
    let lang = st.language.as_str();
    match st.settings_tab {
        2 => crate::i18n::tr(lang, "同一局域网内用浏览器访问该端口").to_string(),
        3 => format!(
            "{}\n{}",
            crate::i18n::tr(lang, "截图保存在 Pictures/CardShot"),
            crate::i18n::tr(lang, "录像保存在 Videos/CardShot")
        ),
        _ => String::new(),
    }
}
