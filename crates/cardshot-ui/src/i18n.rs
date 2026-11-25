//! 界面语言
//!
//! 与 cardbili 一致：源码中的中文原文即翻译键（gettext 的 msgid），译文来自
//! `i18n/<语言>.po`，编译时打包进二进制。缺失条目回退到中文原文，因此新增
//! 文案无需同步修改译文文件也能正常显示

use crate::constants::SettingRow;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

pub const LANGS: [&str; 3] = ["zh", "en", "ja"];

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_current(lang: &str) {
    let index = match code(lang) {
        "en" => 1,
        "ja" => 2,
        _ => 0,
    };
    CURRENT.store(index, Ordering::Relaxed);
}

fn current_name() -> &'static str {
    match CURRENT.load(Ordering::Relaxed) {
        1 => "en",
        2 => "ja",
        _ => "zh",
    }
}

/// 供 Slint 的 I18n.lookup 回调使用
pub fn tr_current(text: &str) -> String {
    tr(current_name(), text)
}

pub fn normalize(lang: &str) -> String {
    code(lang).to_string()
}

fn code(lang: &str) -> &'static str {
    match lang {
        "en" => "en",
        "ja" => "ja",
        _ => "zh",
    }
}

pub fn lang_name(lang: &str) -> String {
    match code(lang) {
        "en" => "English".to_string(),
        "ja" => "日本語".to_string(),
        _ => "简体中文".to_string(),
    }
}

fn catalog(lang: &str) -> Option<&'static HashMap<String, String>> {
    static EN: OnceLock<HashMap<String, String>> = OnceLock::new();
    static JA: OnceLock<HashMap<String, String>> = OnceLock::new();
    match code(lang) {
        "en" => Some(EN.get_or_init(|| parse_po(include_str!("../i18n/en.po")))),
        "ja" => Some(JA.get_or_init(|| parse_po(include_str!("../i18n/ja.po")))),
        _ => None,
    }
}

pub fn tr(lang: &str, text: &str) -> String {
    if let Some(map) = catalog(lang) {
        if let Some(value) = map.get(text) {
            return value.clone();
        }
    }
    text.to_string()
}

fn parse_po(source: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let (mut msgid, mut msgstr) = (String::new(), String::new());
    let mut in_msgstr = false;
    for raw in source.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('"') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("msgid ") {
            if !msgid.is_empty() && !msgstr.is_empty() {
                map.insert(msgid.clone(), msgstr.clone());
            }
            msgid = unquote(rest);
            msgstr.clear();
            in_msgstr = false;
        } else if let Some(rest) = line.strip_prefix("msgstr ") {
            msgstr = unquote(rest);
            in_msgstr = true;
        } else if in_msgstr && line.starts_with('"') {
            msgstr.push_str(&unquote(line));
        }
    }
    if !msgid.is_empty() && !msgstr.is_empty() {
        map.insert(msgid, msgstr);
    }
    map
}

fn unquote(value: &str) -> String {
    let v = value.trim().trim_start_matches('"').trim_end_matches('"');
    v.replace("\\n", "\n").replace("\\\"", "\"")
}

/// 设置文案键：设置项标题与说明
pub fn setting_help(row: SettingRow) -> (&'static str, &'static str) {
    match row {
        SettingRow::Autostart => ("后台常驻", "随系统启动后自动在后台运行，关闭后需手动启动"),
        SettingRow::Language => ("语言设置", "切换界面语言，立即生效"),
        SettingRow::Format => ("图片格式", "截图保存的编码格式，PNG 无损，JPEG 与 WebP 体积更小"),
        SettingRow::Quality => ("压缩质量", "JPEG 与 WebP 的编码质量，数值越高越清晰，文件也越大"),
        SettingRow::Fps => ("录制帧率", "录屏每秒采样的帧数，越高画面越流畅，文件越大"),
        SettingRow::ShotHotkey => ("截图快捷键", "按 Enter 后按下组合键即可保存为截图快捷键"),
        SettingRow::RecordHotkey => ("录制快捷键", "按 Enter 后按下组合键即可保存为录制快捷键"),
        SettingRow::RecordAudio => ("录制声音", "录制扬声器输出（内录），默认关闭"),
        SettingRow::Remote => ("Web管理面板", "开启后可在同一局域网内用浏览器管理相册"),
        SettingRow::Port => ("服务端口", "Web 管理面板监听的端口，按 Enter 手动输入"),
        SettingRow::Clear => ("清空数据", "删除全部截图与录像，此操作不可恢复"),
    }
}

