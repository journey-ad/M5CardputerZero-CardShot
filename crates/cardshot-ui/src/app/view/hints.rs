pub(crate) fn key_width(text: &str) -> f32 {
    let cjk = text.chars().any(|character| (character as u32) > 0x2E80);
    let per = if cjk { 8.5 } else { 5.2 };
    text.chars().count() as f32 * per + 8.0
}

pub(crate) fn hint_items(st: &crate::model::State) -> Vec<crate::HintItem> {
    use crate::constants::{Page, SettingRow};
    let lang = st.language.as_str();
    let translate = |key: &str| crate::i18n::tr(lang, key);
    let label = |text: String| crate::HintItem {
        kind: crate::constants::HINT_TEXT,
        text: text.clone().into(),
        w: key_width(&text) - 2.0,
        vertical: false,
        key: "".into(),
        label: "".into(),
        w2: 0.0,
    };
    let arrow = |vertical: bool| crate::HintItem {
        kind: crate::constants::HINT_ARROW,
        text: "".into(),
        w: 34.0,
        vertical,
        key: "".into(),
        label: "".into(),
        w2: 0.0,
    };
    let stretch = || crate::HintItem {
        kind: crate::constants::HINT_STRETCH,
        text: "".into(),
        w: 0.0,
        vertical: false,
        key: "".into(),
        label: "".into(),
        w2: 0.0,
    };
    let pair = |key: &str, label: &str| crate::HintItem {
        kind: crate::constants::HINT_PAIR,
        text: "".into(),
        w: key_width(&translate(key)),
        vertical: false,
        key: translate(key).into(),
        label: translate(label).into(),
        w2: key_width(&translate(label)) - 2.0,
    };

    if st.page == Page::Settings {
        let row = SettingRow::at(st.settings_focus);
        if st.port_edit {
            return vec![
                label(translate("数字输入")),
                pair("Enter", "保存"),
                pair("Esc", "取消"),
            ];
        }
        if st.hotkey_capture || st.hotkey_record {
            return vec![label(translate("按下组合键")), pair("Esc", "取消")];
        }
        let mut items = vec![arrow(true), label(translate("选择"))];
        if matches!(
            row,
            SettingRow::Format | SettingRow::Quality | SettingRow::Fps
        ) {
            items.push(arrow(false));
            items.push(label(translate("调整")));
        } else {
            let action = match row {
                SettingRow::Autostart | SettingRow::Remote => "开关",
                SettingRow::ShotHotkey | SettingRow::RecordHotkey => "设置",
                SettingRow::Port => "输入",
                SettingRow::Clear => "清空",
                _ => "",
            };
            if !action.is_empty() {
                items.push(pair("Enter", action));
            }
        }
        items.push(stretch());
        items.push(pair("Esc", "返回"));
        return items;
    }

    match st.page {
        Page::Home => vec![pair("Enter", "打开"), stretch(), pair("Esc", "长按退出")],
        Page::Gallery if st.selecting => vec![
            label(format!(
                "{}{}{}",
                translate("已选 "),
                st.selected.len(),
                translate(" 项")
            )),
            stretch(),
            pair("空格", "选择"),
            pair("退格", "删除"),
            pair("Esc", "取消"),
        ],
        Page::Gallery => vec![
            arrow(false),
            label(translate("移动")),
            pair("Enter", "预览"),
            pair("空格", "多选"),
            pair("退格", "删除"),
        ],
        Page::Preview => vec![pair("退格", "删除"), pair("Esc", "返回")],
        Page::Settings => Vec::new(),
    }
}
