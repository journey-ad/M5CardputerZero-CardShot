use slint::ComponentHandle;
// 界面数据装配：缩略图加载、相册模型、属性同步、预览

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use slint::{Rgb8Pixel, SharedPixelBuffer};

use crate::constants::FB_BYTES;
use crate::constants::{SettingRow, GRID_PAGE, PREVIEW_BOX, THUMB_BOX};
use crate::daemon::Client;
use crate::model::{fmt_clock, fmt_duration, human_size, State, VideoPlayer};
use crate::{AppWindow, GalleryItem};

mod hints;
mod preview;
mod settings;

use hints::hint_items;
pub use preview::*;
use settings::{setting_footer, setting_rows, setting_tabs};

fn expand_home(path: &str) -> std::path::PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return Path::new(&home).join(rest);
        }
    }
    Path::new(path).to_path_buf()
}

fn load_sized(path: &Path, max: (u32, u32)) -> Option<slint::Image> {
    let img = image::open(path).ok()?.to_rgb8();
    let (width, height) = (img.width(), img.height());
    if width == 0 || height == 0 {
        return None;
    }
    // 在目标尺寸内时使用原始像素
    let (nw, nh) = if width <= max.0 && height <= max.1 {
        (width, height)
    } else {
        let scale = (max.0 as f64 / width as f64).min(max.1 as f64 / height as f64);
        (
            ((width as f64 * scale).round() as u32).max(1),
            ((height as f64 * scale).round() as u32).max(1),
        )
    };
    let resized = if (nw, nh) == (width, height) {
        img
    } else {
        image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle)
    };

    let mut buf = SharedPixelBuffer::<Rgb8Pixel>::new(nw, nh);
    let dst = buf.make_mut_slice();
    let raw = resized.as_raw();
    for index in 0..(nw * nh) as usize {
        dst[index] = Rgb8Pixel {
            r: raw[index * 3],
            g: raw[index * 3 + 1],
            b: raw[index * 3 + 2],
        };
    }
    Some(slint::Image::from_rgb8(buf))
}

fn item_image(client: &mut Client, st: &mut State, idx: usize, big: bool) -> Option<slint::Image> {
    let it = st.items.get(idx)?.clone();
    let cache: &mut HashMap<String, slint::Image> = if big {
        &mut st.images
    } else {
        &mut st.images_grid
    };
    let key = if big {
        format!("{}@big", it.id)
    } else {
        it.id.clone()
    };
    if let Some(img) = cache.get(&key) {
        return Some(img.clone());
    }
    let box_size = if big { PREVIEW_BOX } else { THUMB_BOX };
    let img = if big && it.kind == "image" && !st.output_dir.is_empty() {
        let path = expand_home(&st.output_dir).join(&it.name);
        load_sized(&path, box_size).or_else(|| {
            client
                .thumb_path(&it.id)
                .and_then(|path| load_sized(&path, box_size))
        })
    } else {
        client
            .thumb_path(&it.id)
            .and_then(|path| load_sized(&path, box_size))
    }?;
    cache.insert(key, img.clone());
    Some(img)
}

pub fn set_gallery_model(ui: &AppWindow, client: &mut Client, st: &mut State) {
    let mut model: Vec<GalleryItem> = Vec::new();
    let start = st.scroll.min(st.items.len());
    let end = (start + GRID_PAGE).min(st.items.len());
    for idx in start..end {
        let it = st.items[idx].clone();
        let img = item_image(client, st, idx, false).unwrap_or_default();
        let duration = it.duration.map(fmt_duration).unwrap_or_default();
        model.push(GalleryItem {
            img,
            kind: it.kind.clone().into(),
            name: it.name.clone().into(),
            sub: human_size(it.size).into(),
            dur: duration.into(),
        });
    }
    ui.set_items(slint::ModelRc::new(slint::VecModel::from(model)));
}

pub fn sync_ui(ui: &AppWindow, st: &State) {
    ui.set_page(st.page.index());
    ui.set_home_focus(st.home_focus as i32);
    ui.set_grid_focus(st.grid_focus.saturating_sub(st.scroll) as i32);
    ui.set_selecting(st.selecting);
    ui.set_sel_count(st.selected.len() as i32);
    let start = st.scroll.min(st.items.len());
    let end = (start + GRID_PAGE).min(st.items.len());
    let flags: Vec<bool> = (start..end)
        .map(|index| st.selected.contains(&index))
        .collect();
    ui.set_selected_flags(slint::ModelRc::new(slint::VecModel::from(flags)));
    ui.set_recording(st.recording);
    ui.set_rec_time(fmt_clock(st.rec_seconds as f64).into());
    if st.page == crate::constants::Page::Preview {
        ui.set_page_title(st.preview_name.clone().into());
    } else {
        ui.set_page_title(crate::i18n::tr(&st.language, st.page.title()).into());
    }
    let row = SettingRow::at(st.settings_focus);
    let lang = st.language.as_str();
    ui.set_hints(slint::ModelRc::new(slint::VecModel::from(hint_items(st))));
    ui.set_set_tab(st.settings_tab as i32);
    ui.set_setting_help_visible(st.setting_help_visible);
    if st.setting_help_visible {
        let (lk, dk) = crate::i18n::setting_help(row);
        ui.set_s_setting_help_title(crate::i18n::tr(lang, lk).into());
        ui.set_s_setting_help_body(crate::i18n::tr(lang, dk).into());
    }
    crate::i18n::set_current(lang);
    ui.global::<crate::I18n>().set_lang(if lang == "en" {
        1
    } else if lang == "ja" {
        2
    } else {
        0
    });
    ui.set_count_images(st.count_images as i32);
    ui.set_count_videos(st.count_videos as i32);
    ui.set_size_label(human_size(st.total_bytes).into());
    ui.set_setting_tabs(slint::ModelRc::new(slint::VecModel::from(setting_tabs(st))));
    ui.set_setting_rows(slint::ModelRc::new(slint::VecModel::from(setting_rows(st))));
    ui.set_setting_footer(setting_footer(st).into());
    ui.set_help_visible(st.help_visible);
    ui.set_confirm_visible(st.confirm_visible);
    ui.set_confirm_focus(st.confirm_focus);
    ui.set_preview_pos(st.preview_pos.clone().into());
    ui.set_video_ended(st.video_ended);
    if st.confirm_kind == crate::constants::CONFIRM_CLEAR {
        ui.set_confirm_title("清空全部数据".into());
        ui.set_confirm_desc("确定删除所有截图与录制？删除后无法恢复".into());
    } else {
        ui.set_confirm_title("删除当前内容".into());
        ui.set_confirm_desc("是否确认进行删除？删除后将无法恢复".into());
    }
    ui.set_toast(st.toast_text().into());
}
