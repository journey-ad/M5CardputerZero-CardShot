// 从守护拉取状态、配置与列表

use crate::daemon::Client;
use crate::model::State;

pub fn poll_status(client: &mut Client, st: &mut State) {
    let Some(status) = client.status() else {
        return;
    };
    if let Some(number) = status
        .pointer("/counts/images")
        .and_then(|value| value.as_u64())
    {
        st.count_images = number as usize;
    }
    if let Some(number) = status
        .pointer("/counts/videos")
        .and_then(|value| value.as_u64())
    {
        st.count_videos = number as usize;
    }
    if let Some(number) = status.get("total_bytes").and_then(|value| value.as_u64()) {
        st.total_bytes = number;
    }
    if let Some(flag) = status.get("recording").and_then(|value| value.as_bool()) {
        st.recording = flag;
    }
    if let Some(number) = status.get("elapsed").and_then(|value| value.as_f64()) {
        st.rec_seconds = number as u64;
    }
    if let Some(format) = status.get("image_format").and_then(|value| value.as_str()) {
        st.image_format = format.to_string();
    }
    if let Some(quality) = status.get("image_quality").and_then(|value| value.as_i64()) {
        st.image_quality = quality;
    }
    if let Some(dir) = status.get("output_dir").and_then(|value| value.as_str()) {
        st.output_dir = dir.to_string();
    }
    if let Some(dir) = status.get("video_dir").and_then(|value| value.as_str()) {
        st.video_dir = dir.to_string();
    }
    if let Some(remote) = status.get("remote") {
        if let Some(flag) = remote.get("running").and_then(|value| value.as_bool()) {
            st.remote_running = flag;
        }
        if let Some(port) = remote.get("port").and_then(|value| value.as_i64()) {
            st.remote_port = port;
        }
        if let Some(host) = remote.get("host").and_then(|value| value.as_str()) {
            st.remote_host = host.to_string();
        }
    }
}

pub fn poll_config(client: &mut Client, st: &mut State) {
    let Some(config) = client.config() else {
        return;
    };
    if let Some(format) = config.get("image_format").and_then(|value| value.as_str()) {
        st.image_format = format.to_string();
    }
    if let Some(quality) = config.get("image_quality").and_then(|value| value.as_i64()) {
        st.image_quality = quality;
    }
    if let Some(format) = config
        .pointer("/record/fps")
        .and_then(|value| value.as_i64())
    {
        st.fps = format;
    }
    if let Some(flag) = config
        .pointer("/record/audio")
        .and_then(|value| value.as_bool())
    {
        st.record_audio = flag;
    }
    if let Some(language) = config.pointer("/language").and_then(|value| value.as_str()) {
        st.language = crate::i18n::normalize(language).to_string();
    }
    if let Some(autostart) = config
        .pointer("/service/autostart")
        .and_then(|value| value.as_bool())
    {
        st.autostart = autostart;
    }
    if let Some(enabled) = config
        .pointer("/remote/enabled")
        .and_then(|value| value.as_bool())
    {
        st.remote_running = enabled;
    }
    if let Some(port) = config
        .pointer("/remote/port")
        .and_then(|value| value.as_i64())
    {
        st.remote_port = port;
    }
    if let Some(host) = config
        .pointer("/hotkeys/enabled")
        .and_then(|value| value.as_bool())
    {
        st.hotkey_enabled = host;
    }
    if let Some(key) = config
        .pointer("/hotkeys/shot")
        .and_then(|value| value.as_str())
    {
        st.shot_hotkey = key.to_string();
    }
    if let Some(rk) = config
        .pointer("/hotkeys/record")
        .and_then(|value| value.as_str())
    {
        st.record_hotkey = rk.to_string();
    }
}

pub fn load_items(client: &mut Client, st: &mut State) {
    st.items = client.items();
    st.images.clear();
    st.images_grid.clear();
    st.selected.clear();
    if st.grid_focus >= st.items.len() {
        st.grid_focus = st.items.len().saturating_sub(1);
    }
    st.fix_scroll();
}
