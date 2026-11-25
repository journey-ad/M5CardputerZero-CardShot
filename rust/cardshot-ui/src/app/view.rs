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

use crate::constants::{PREVIEW_BOX, THUMB_BOX, GRID_PAGE, SettingRow};
use crate::daemon::Client;
use crate::constants::FB_BYTES;
use crate::model::{fmt_clock, fmt_duration, human_size, State, VideoPlayer};
use crate::{AppWindow, GalleryItem};

fn expand_home(p: &str) -> std::path::PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return Path::new(&home).join(rest);
        }
    }
    Path::new(p).to_path_buf()
}

fn load_sized(path: &Path, max: (u32, u32)) -> Option<slint::Image> {
    let img = image::open(path).ok()?.to_rgb8();
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return None;
    }
    // 已在目标尺寸内则直接使用原始像素，避免重采样造成模糊
    let (nw, nh) = if w <= max.0 && h <= max.1 {
        (w, h)
    } else {
        let scale = (max.0 as f64 / w as f64).min(max.1 as f64 / h as f64);
        (((w as f64 * scale).round() as u32).max(1), ((h as f64 * scale).round() as u32).max(1))
    };
    let resized = if (nw, nh) == (w, h) {
        img
    } else {
        image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle)
    };

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
        let p = expand_home(&st.output_dir).join(&it.name);
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
    ui.set_rec_time(fmt_clock(st.rec_seconds as f64).into());
    if st.page == crate::constants::Page::Preview {
        ui.set_page_title(st.preview_name.clone().into());
    } else {
        ui.set_page_title(crate::i18n::tr(&st.language, st.page.title()).into());
    }
    let settings = st.page == crate::constants::Page::Settings;
    let row = SettingRow::at(st.settings_focus);
    let adjust = settings && matches!(row, SettingRow::Format | SettingRow::Quality | SettingRow::Fps);
    let editing = settings && (st.port_edit || st.hotkey_capture);
    let lang = st.language.as_str();
    let hint: String = if settings && st.port_edit {
        crate::i18n::tr(lang, "数字输入   Enter 保存   Esc 取消")
    } else if settings && st.hotkey_capture {
        crate::i18n::tr(lang, "按下组合键   Esc 取消")
    } else if settings {
        match row {
            SettingRow::Autostart => crate::i18n::tr(lang, "Enter 开关"),
            SettingRow::ShotHotkey | SettingRow::RecordHotkey => crate::i18n::tr(lang, "Enter 设置"),
            SettingRow::Remote => crate::i18n::tr(lang, "Enter 开关"),
            SettingRow::Port => crate::i18n::tr(lang, "Enter 输入"),
            SettingRow::Clear => crate::i18n::tr(lang, "Enter 清空"),
            _ => String::new(),
        }
    } else {
        crate::i18n::tr(lang, st.page.hint())
    };
    ui.set_hint(hint.into());
    ui.set_set_tab(st.settings_tab as i32);
    ui.set_s_autostart(if st.autostart { crate::i18n::tr(lang, "开").into() } else { crate::i18n::tr(lang, "关").into() });
    ui.set_s_language(crate::i18n::lang_name(&st.language).into());
    ui.set_setting_help_visible(st.setting_help_visible);
    if st.setting_help_visible {
        let (lk, dk) = crate::i18n::setting_help(row);
        ui.set_s_setting_help_title(crate::i18n::tr(lang, lk).into());
        ui.set_s_setting_help_body(crate::i18n::tr(lang, dk).into());
    }
    crate::i18n::set_current(lang);
    ui.global::<crate::I18n>().set_lang(if lang == "en" { 1 } else if lang == "ja" { 2 } else { 0 });
    ui.set_settings_adjust(adjust);
    ui.set_settings_editing(editing);
    ui.set_count_images(st.count_images as i32);
    ui.set_count_videos(st.count_videos as i32);
    ui.set_size_label(human_size(st.total_bytes).into());
    ui.set_s_fmt(st.image_format.to_uppercase().into());
    ui.set_s_quality(st.image_quality as i32);
    ui.set_s_fps(st.fps as i32);
    ui.set_s_port(st.remote_port as i32);
    ui.set_s_port_text(if st.port_edit {
        format!("{}_", st.port_buf).into()
    } else if st.remote_host.is_empty() {
        st.remote_port.to_string().into()
    } else {
        format!("{}:{}", st.remote_host, st.remote_port).into()
    });
    ui.set_s_port_host(if st.remote_host.is_empty() { String::new().into() } else { format!("{}:", st.remote_host).into() });
    ui.set_s_port_value(if st.port_edit { format!("{}_", st.port_buf).into() } else { st.remote_port.to_string().into() });
    ui.set_port_edit(st.port_edit);
    ui.set_port_disabled(!st.remote_running);
    let shot_text = if st.hotkey_capture && !st.hotkey_record {
        if st.hotkey_buffer.is_empty() {
            "按下组合键…".to_string()
        } else {
            st.hotkey_buffer.clone()
        }
    } else {
        st.shot_hotkey.clone()
    };
    ui.set_s_shot(shot_text.into());
    let record_text = if st.hotkey_capture && st.hotkey_record {
        if st.hotkey_buffer.is_empty() { "按下组合键…".to_string() } else { st.hotkey_buffer.clone() }
    } else {
        st.record_hotkey.clone()
    };
    ui.set_s_record(record_text.into());
    ui.set_hotkey_capture(st.hotkey_capture);
    ui.set_s_record_audio(if st.record_audio { crate::i18n::tr(lang, "开").into() } else { crate::i18n::tr(lang, "关").into() });
    ui.set_s_remote(if st.remote_running { crate::i18n::tr(lang, "开").into() } else { crate::i18n::tr(lang, "关").into() });
    ui.set_help_visible(st.help_visible);
    ui.set_confirm_visible(st.confirm_visible);
    ui.set_confirm_focus(st.confirm_focus);
    ui.set_preview_pos(st.preview_pos.clone().into());
    ui.set_video_ended(st.video_ended);
    if st.confirm_kind == 1 {
        ui.set_confirm_title("清空全部数据".into());
        ui.set_confirm_desc("确定删除所有截图与录制？删除后无法恢复".into());
    } else {
        ui.set_confirm_title("删除当前内容".into());
        ui.set_confirm_desc("是否确认进行删除？删除后将无法恢复".into());
    }
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
    if it.kind == "video" {
        if std::env::var("CARDSHOT_TEST_ENDED").is_ok() {
            stop_video(st);
            st.video_ended = true;
            ui.set_video_ended(true);
        } else {
            let base = if st.video_dir.is_empty() { st.output_dir.clone() } else { st.video_dir.clone() };
            let path = expand_home(&base).join(&it.name);
            start_video(st, &path, st.fps.max(1) as u32);
        }
    } else {
        stop_video(st);
    }
}

/// 停止当前视频解码线程
pub fn stop_video(st: &mut State) {
    if let Some(v) = st.video.take() {
        v.stop.store(true, Ordering::Relaxed);
    }
    st.video_ended = false;
}

/// 启动视频预览：后台 ffmpeg 按实时速度解码为 320x170 rgb565 帧
fn start_video(st: &mut State, path: &Path, fps: u32) {
    stop_video(st);
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let stop = Arc::new(AtomicBool::new(false));
    let ended = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    let ended_thread = ended.clone();
    let p = path.to_path_buf();
    std::thread::spawn(move || {
        eprintln!("video: decoding {:?}", p);
        let mut child = match Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-re", "-i"])
            .arg(&p)
            .args([
                "-f", "rawvideo", "-pix_fmt", "rgb565le", "-s", "320x170", "-r",
                &fps.to_string(), "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => { eprintln!("video: spawn failed {}", e); return; }
        };
        if let Some(mut out) = child.stdout.take() {
            let mut buf = vec![0u8; FB_BYTES];
            while !stop_thread.load(Ordering::Relaxed) {
                if let Err(e) = out.read_exact(&mut buf) {
                    eprintln!("video: read end {}", e);
                    if !stop_thread.load(Ordering::Relaxed) {
                        ended_thread.store(true, Ordering::Relaxed);
                    }
                    break;
                }
                if tx.send(buf.clone()).is_err() {
                    eprintln!("video: receiver gone");
                    break;
                }
            }
        }
        let _ = child.kill();
    });
    let interval = Duration::from_millis((1000 / fps.max(1) as u64).max(10));
    st.video = Some(VideoPlayer { rx, stop, ended, next_at: Instant::now(), interval });
    st.video_ended = false;
}

/// 每帧调用：从队列取最新一帧刷新预览图
pub fn tick_video(ui: &AppWindow, st: &mut State) {
    let now = Instant::now();
    let (ended, latest, interval) = {
        let Some(v) = st.video.as_mut() else { return };
        if now < v.next_at {
            return;
        }
        let mut latest: Option<Vec<u8>> = None;
        while let Ok(f) = v.rx.try_recv() {
            latest = Some(f);
        }
        (v.ended.load(Ordering::Relaxed), latest, v.interval)
    };
    if ended && latest.is_none() {
        st.video_ended = true;
        ui.set_video_ended(true);
        return;
    }
    let Some(raw) = latest else { return };
    let mut buf = SharedPixelBuffer::<Rgb8Pixel>::new(320, 170);
    let dst = buf.make_mut_slice();
    for i in 0..dst.len() {
        let v = (raw[i * 2] as u16) | ((raw[i * 2 + 1] as u16) << 8);
        let r = ((v >> 11) & 0x1f) as u8;
        let g = ((v >> 5) & 0x3f) as u8;
        let b = (v & 0x1f) as u8;
        dst[i] = Rgb8Pixel { r: (r << 3) | (r >> 2), g: (g << 2) | (g >> 4), b: (b << 3) | (b >> 2) };
    }
    ui.set_preview_img(slint::Image::from_rgb8(buf));
    if let Some(v) = st.video.as_mut() {
        v.next_at = now + interval;
    }
}



