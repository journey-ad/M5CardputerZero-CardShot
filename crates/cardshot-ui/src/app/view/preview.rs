use super::*;

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
            let base = if st.video_dir.is_empty() {
                st.output_dir.clone()
            } else {
                st.video_dir.clone()
            };
            let path = expand_home(&base).join(&it.name);
            start_video(st, &path, st.fps.max(1) as u32);
        }
    } else {
        stop_video(st);
    }
}

/// 停止当前视频解码线程

pub fn stop_video(st: &mut State) {
    if let Some(video) = st.video.take() {
        video.stop.store(true, Ordering::Relaxed);
    }
    st.video_ended = false;
}

/// 启动视频预览：后台 ffmpeg 按实时速度解码为 320x170 rgb565 帧

pub(crate) fn start_video(st: &mut State, path: &Path, fps: u32) {
    stop_video(st);
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let stop = Arc::new(AtomicBool::new(false));
    let ended = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    let ended_thread = ended.clone();
    let path = path.to_path_buf();
    std::thread::spawn(move || {
        eprintln!("video: decoding {:?}", path);
        let mut child = match Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-re", "-i"])
            .arg(&path)
            .args([
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb565le",
                "-s",
                "320x170",
                "-r",
                &fps.to_string(),
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                eprintln!("video: spawn failed {}", error);
                return;
            }
        };
        if let Some(mut out) = child.stdout.take() {
            let mut buf = vec![0u8; FB_BYTES];
            while !stop_thread.load(Ordering::Relaxed) {
                if let Err(error) = out.read_exact(&mut buf) {
                    eprintln!("video: read end {}", error);
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
    st.video = Some(VideoPlayer {
        rx,
        stop,
        ended,
        next_at: Instant::now(),
        interval,
    });
    st.video_ended = false;
}

/// 每帧调用：从队列取最新一帧刷新预览图

pub fn tick_video(ui: &AppWindow, st: &mut State) {
    let now = Instant::now();
    let (ended, latest, interval) = {
        let Some(video) = st.video.as_mut() else {
            return;
        };
        if now < video.next_at {
            return;
        }
        let mut latest: Option<Vec<u8>> = None;
        while let Ok(frame) = video.rx.try_recv() {
            latest = Some(frame);
        }
        (video.ended.load(Ordering::Relaxed), latest, video.interval)
    };
    if ended && latest.is_none() {
        st.video_ended = true;
        ui.set_video_ended(true);
        return;
    }
    let Some(raw) = latest else { return };
    let mut buf = SharedPixelBuffer::<Rgb8Pixel>::new(320, 170);
    let dst = buf.make_mut_slice();
    for index in 0..dst.len() {
        let video = (raw[index * 2] as u16) | ((raw[index * 2 + 1] as u16) << 8);
        let red = ((video >> 11) & 0x1f) as u8;
        let green = ((video >> 5) & 0x3f) as u8;
        let blue = (video & 0x1f) as u8;
        dst[index] = Rgb8Pixel {
            r: (red << 3) | (red >> 2),
            g: (green << 2) | (green >> 4),
            b: (blue << 3) | (blue >> 2),
        };
    }
    ui.set_preview_img(slint::Image::from_rgb8(buf));
    if let Some(video) = st.video.as_mut() {
        video.next_at = now + interval;
    }
}
