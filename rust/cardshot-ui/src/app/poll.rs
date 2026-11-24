// 从守护拉取状态、配置与列表

use crate::daemon::Client;
use crate::model::State;

pub fn poll_status(client: &mut Client, st: &mut State) {
    let Some(s) = client.status() else { return };
    if let Some(n) = s.pointer("/counts/images").and_then(|x| x.as_u64()) {
        st.count_images = n as usize;
    }
    if let Some(n) = s.pointer("/counts/videos").and_then(|x| x.as_u64()) {
        st.count_videos = n as usize;
    }
    if let Some(n) = s.get("total_bytes").and_then(|x| x.as_u64()) {
        st.total_bytes = n;
    }
    if let Some(b) = s.get("recording").and_then(|x| x.as_bool()) {
        st.recording = b;
    }
    if let Some(n) = s.get("elapsed").and_then(|x| x.as_f64()) {
        st.rec_seconds = n as u64;
    }
    if let Some(f) = s.get("image_format").and_then(|x| x.as_str()) {
        st.image_format = f.to_string();
    }
    if let Some(q) = s.get("image_quality").and_then(|x| x.as_i64()) {
        st.image_quality = q;
    }
    if let Some(d) = s.get("output_dir").and_then(|x| x.as_str()) {
        st.output_dir = d.to_string();
    }
    if let Some(d) = s.get("video_dir").and_then(|x| x.as_str()) {
        st.video_dir = d.to_string();
    }
    if let Some(r) = s.get("remote") {
        if let Some(b) = r.get("running").and_then(|x| x.as_bool()) {
            st.remote_running = b;
        }
        if let Some(p) = r.get("port").and_then(|x| x.as_i64()) {
            st.remote_port = p;
        }
        if let Some(h) = r.get("host").and_then(|x| x.as_str()) {
            st.remote_host = h.to_string();
        }
    }
}

pub fn poll_config(client: &mut Client, st: &mut State) {
    let Some(c) = client.config() else { return };
    if let Some(f) = c.get("image_format").and_then(|x| x.as_str()) {
        st.image_format = f.to_string();
    }
    if let Some(q) = c.get("image_quality").and_then(|x| x.as_i64()) {
        st.image_quality = q;
    }
    if let Some(f) = c.pointer("/record/fps").and_then(|x| x.as_i64()) {
        st.fps = f;
    }
    if let Some(a) = c.pointer("/service/autostart").and_then(|x| x.as_bool()) {
        st.autostart = a;
    }
    if let Some(e) = c.pointer("/remote/enabled").and_then(|x| x.as_bool()) {
        st.remote_running = e;
    }
    if let Some(p) = c.pointer("/remote/port").and_then(|x| x.as_i64()) {
        st.remote_port = p;
    }
    if let Some(h) = c.pointer("/hotkeys/enabled").and_then(|x| x.as_bool()) {
        st.hotkey_enabled = h;
    }
    if let Some(k) = c.pointer("/hotkeys/shot").and_then(|x| x.as_str()) {
        st.shot_hotkey = k.to_string();
    }
    if let Some(rk) = c.pointer("/hotkeys/record").and_then(|x| x.as_str()) {
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
