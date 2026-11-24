// 界面数据装配：缩略图加载、相册模型、属性同步、预览

use std::collections::HashMap;
use std::path::Path;

use slint::{Rgb8Pixel, SharedPixelBuffer};

use crate::constants::{PREVIEW_BOX, THUMB_BOX, GRID_PAGE};
use crate::daemon::Client;
use crate::model::{fmt_duration, human_size, State};
use crate::{AppWindow, GalleryItem};

fn load_sized(path: &Path, max: (u32, u32)) -> Option<slint::Image> {
    let img = image::open(path).ok()?.to_rgb8();
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return None;
    }
    let scale = (max.0 as f64 / w as f64).min(max.1 as f64 / h as f64);
    let nw = ((w as f64 * scale).round() as u32).max(1);
    let nh = ((h as f64 * scale).round() as u32).max(1);
    let resized = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle);

    let mut buf = SharedPixelBuffer::<Rgb8Pixel>::new(nw, nh);
    let dst = buf.make_mut_slice();
    let raw = resized.as_raw();
    for i in 0..(nw * nh) as usize {
        dst[i] = Rgb8Pixel {
            r: raw[i * 3],
            g: raw[i * 3 + 1],
            b: raw[i * 3 + 2],
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
    let key = if big { format!("{}@big", it.id) } else { it.id.clone() };
    if let Some(img) = cache.get(&key) {
        return Some(img.clone());
    }
    let box_size = if big { PREVIEW_BOX } else { THUMB_BOX };
    let img = if big && it.kind == "image" && !st.output_dir.is_empty() {
        let p = Path::new(&st.output_dir).join(&it.name);
        load_sized(&p, box_size)
            .or_else(|| client.thumb_path(&it.id).and_then(|p| load_sized(&p, box_size)))
    } else {
        client.thumb_path(&it.id).and_then(|p| load_sized(&p, box_size))
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
    ui.set_set_focus(st.settings_focus as i32);
    ui.set_recording(st.recording);
    ui.set_rec_time(fmt_duration(st.rec_seconds as f64).into());
    if st.page == crate::constants::Page::Preview {
        ui.set_page_title(st.preview_name.clone().into());
    } else {
        ui.set_page_title(st.page.title().into());
    }
    ui.set_hint(st.page.hint().into());
    ui.set_count_images(st.count_images as i32);
    ui.set_count_videos(st.count_videos as i32);
    ui.set_size_label(human_size(st.total_bytes).into());
    ui.set_s_fmt(st.image_format.to_uppercase().into());
    ui.set_s_quality(st.image_quality as i32);
    ui.set_s_fps(st.fps as i32);
    ui.set_s_port(st.remote_port as i32);
    ui.set_s_shot(st.shot_hotkey.clone().into());
    ui.set_s_remote(if st.remote_running { "开".into() } else { "关".into() });
    ui.set_s_hotkey(if st.hotkey_enabled { "开".into() } else { "关".into() });
    ui.set_help_visible(st.help_visible);
    ui.set_confirm_visible(st.confirm_visible);
    ui.set_confirm_focus(st.confirm_focus);
    ui.set_preview_pos(st.preview_pos.clone().into());
    ui.set_toast(st.toast_text().into());
}

pub fn open_preview(ui: &AppWindow, client: &mut Client, st: &mut State) {
    if st.items.is_empty() {
        return;
    }
    let idx = st.grid_focus.min(st.items.len() - 1);
    let it = st.items[idx].clone();
    let img = item_image(client, st, idx, true).unwrap_or_default();
    let duration = it.duration.map(fmt_duration).unwrap_or_default();
    ui.set_preview_img(img);
    ui.set_preview_name(it.name.clone().into());
    ui.set_preview_size(human_size(it.size).into());
    ui.set_preview_dur(duration.clone().into());
    ui.set_preview_kind(it.kind.clone().into());
    st.preview_name = it.name.clone();
    st.preview_size = human_size(it.size);
    st.preview_dur = duration;
    st.preview_kind = it.kind.clone();
    st.preview_pos = format!("{}/{}", idx + 1, st.items.len());
    st.page = crate::constants::Page::Preview;
}
