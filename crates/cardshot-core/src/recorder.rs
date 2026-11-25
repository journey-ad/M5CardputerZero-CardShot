use crate::capture;
use crate::config::Config;
use crate::util::{expand_home, local_time_fmt};
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Serialize, Clone, Default)]
pub struct RecState {
    pub recording: bool,
    pub elapsed: f64,
    pub frames: u64,
    pub current_file: Option<String>,
    pub last_error: Option<String>,
}

pub struct Recorder {
    state: Arc<Mutex<RecState>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    started: Option<Instant>,
    path: Option<PathBuf>,
}

impl Recorder {
    pub fn new() -> Self {
        Recorder {
            state: Arc::new(Mutex::new(RecState::default())),
            stop: Arc::new(AtomicBool::new(false)),
            handle: None,
            started: None,
            path: None,
        }
    }

    pub fn status(&self) -> RecState {
        let mut st = self.state.lock().unwrap().clone();
        if st.recording {
            if let Some(s) = self.started {
                st.elapsed = s.elapsed().as_secs_f64();
            }
        }
        st
    }

    pub fn is_recording(&self) -> bool {
        self.state.lock().unwrap().recording
    }

    pub fn start(&mut self, cfg: &Config) -> Result<PathBuf, String> {
        if self.is_recording() {
            return Err("already recording".into());
        }
        let dir = expand_home(&cfg.video_dir);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let name = format!("rec_{}.mp4", local_time_fmt("%Y%m%d_%H%M%S"));
        let path = dir.join(&name);
        let fps = cfg.record.fps.max(1).to_string();

        let encoder = cfg.record.encoder.clone();
        let bitrate = cfg.record.bitrate.clone();
        let audio = cfg.record.audio;
        let audio_dev = cfg.record.audio_device.clone();
        let logpath = dir.join("ffmpeg_rec.log");

        let stamp = format!("{}_{}", std::process::id(), local_time_fmt("%Y%m%d_%H%M%S"));
        let tmpdir = std::env::temp_dir();
        let vtmp = tmpdir.join(format!("cardshot_v_{stamp}.mp4"));
        let atmp = tmpdir.join(format!("cardshot_a_{stamp}.wav"));

        let mut cmd = Command::new("ffmpeg");
        cmd.args([
            "-hide_banner", "-loglevel", "error", "-y", "-f", "rawvideo", "-pix_fmt",
            "rgb565le", "-s", "320x170", "-r", &fps, "-i", "-",
            "-an", "-c:v", &encoder, "-b:v", &bitrate, "-pix_fmt", "yuv420p",
        ]);
        if encoder == "libx264" {
            cmd.args(["-preset", "ultrafast", "-tune", "zerolatency"]);
        }
        cmd.arg("-movflags").arg("+faststart").arg(&vtmp);
        let logf = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&logpath)
            .ok();
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(match logf {
                Some(f) => Stdio::from(f),
                None => Stdio::null(),
            });

        let mut child = cmd.spawn().map_err(|e| format!("spawn ffmpeg: {e}"))?;
        let mut stdin = child.stdin.take().unwrap();

        // 音频单独录制，停止后再与视频合成，避免双输入在停止时互相阻塞
        let mut audio_child = None;
        if audio {
            let mut ac = Command::new("ffmpeg");
            ac.args([
                "-hide_banner", "-loglevel", "error", "-y", "-f", "pulse",
                "-thread_queue_size", "1024", "-i", &audio_dev, "-ar", "48000", "-ac", "2",
                "-c:a", "pcm_s16le",
            ])
            .arg(&atmp)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
            let alog = std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(dir.join("audio_rec.log")).ok();
            if let Some(f) = alog { ac.stderr(Stdio::from(f)); }
            match ac.spawn() {
                Ok(c) => audio_child = Some(c),
                Err(e) => {
                    self.state.lock().unwrap().last_error = Some(format!("spawn audio: {e}"));
                }
            }
        }

        // 预滚一帧：穿过硬件编码器初始化，避免开头长时间无帧
        let first = capture::read_frame().map_err(|e| format!("read fb: {e}"))?;
        if let Err(e) = stdin.write_all(&first) {
            let _ = child.wait();
            return Err(format!("prime write: {e}"));
        }

        self.stop.store(false, Ordering::SeqCst);
        {
            let mut s = self.state.lock().unwrap();
            *s = RecState {
                recording: true,
                elapsed: 0.0,
                frames: 1,
                current_file: Some(path.to_string_lossy().into_owned()),
                last_error: None,
            };
        }

        let st = self.state.clone();
        let stop = self.stop.clone();
        let interval = Duration::from_secs_f64(1.0 / cfg.record.fps.max(1) as f64);
        let max_secs = cfg.record.max_seconds.max(1);
        let deadline = Instant::now() + Duration::from_secs(max_secs);
        let path_thread = path.clone();
        let handle = thread::spawn(move || {
            let mut frames = 1u64;
            loop {
                if stop.load(Ordering::SeqCst) || Instant::now() >= deadline {
                    break;
                }
                match capture::read_frame() {
                    Ok(mut raw) => {
                        crate::overlay::apply_mask(&mut raw);
                        if let Err(err) = stdin.write_all(&raw) {
                            st.lock().unwrap().last_error = Some(format!("write: {err}"));
                            break;
                        }
                        frames += 1;
                        st.lock().unwrap().frames = frames;
                    }
                    Err(e) => {
                        st.lock().unwrap().last_error = Some(e.to_string());
                    }
                }
                thread::sleep(interval);
            }
            drop(stdin);

            let video_ok = finish_child(&mut child);
            if let Some(mut ac) = audio_child.take() {
                let _ = finish_child(&mut ac);
            }
            let audio_ok = atmp.metadata().map(|m| m.len() > 1000).unwrap_or(false);

            let mut err = None;
            if !video_ok {
                err = Some("ffmpeg 未正常结束".to_string());
            }
            if audio && audio_ok && atmp.exists() && vtmp.exists() {
                let mut mux = Command::new("ffmpeg");
                mux.args([
                    "-hide_banner", "-loglevel", "error", "-y", "-i",
                ])
                .arg(&vtmp)
                .arg("-i")
                .arg(&atmp)
                .args(["-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "aac", "-b:a", "96k", "-shortest"])
                .arg(&path_thread)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
                match mux.status() {
                    Ok(s2) if s2.success() => {
                        let _ = std::fs::remove_file(&vtmp);
                        let _ = std::fs::remove_file(&atmp);
                    }
                    _ => {
                        let _ = move_file(&vtmp, &path_thread);
                        err = Some("音视频合成失败，已保存无声视频".to_string());
                    }
                }
            } else if vtmp.exists() {
                let _ = move_file(&vtmp, &path_thread);
                let _ = std::fs::remove_file(&atmp);
                if audio && !audio_ok {
                    err = Some("音频录制失败，已保存无声视频".to_string());
                }
            } else {
                err = Some("未生成视频文件".to_string());
            }

            let mut s = st.lock().unwrap();
            s.recording = false;
            s.current_file = None;
            s.elapsed = 0.0;
            if let Some(e) = err {
                s.last_error = Some(e);
            }
        });

        self.handle = Some(handle);
        self.started = Some(Instant::now());
        self.path = Some(path.clone());
        Ok(path)
    }

    pub fn stop(&mut self) -> Result<PathBuf, String> {
        if !self.is_recording() {
            if let Some(p) = self.path.clone() {
                return Ok(p);
            }
            return Err("not recording".into());
        }
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        self.started = None;
        self.path.clone().ok_or_else(|| "no path".into())
    }

    pub fn toggle(&mut self, cfg: &Config) -> Result<(bool, PathBuf), String> {
        if self.is_recording() {
            let p = self.stop()?;
            Ok((false, p))
        } else {
            let p = self.start(cfg)?;
            Ok((true, p))
        }
    }
}

fn finish_child(child: &mut std::process::Child) -> bool {
    let grace = Instant::now() + Duration::from_millis(800);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {
                if Instant::now() > grace {
                    unsafe {
                        libc::kill(child.id() as i32, libc::SIGINT);
                    }
                    thread::sleep(Duration::from_millis(600));
                    return match child.try_wait() {
                        Ok(Some(status)) => status.success(),
                        _ => {
                            let _ = child.kill();
                            let _ = child.wait();
                            false
                        }
                    };
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return false,
        }
    }
}

fn move_file(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)?;
    let _ = std::fs::remove_file(from);
    Ok(())
}
