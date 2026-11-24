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

        let mut cmd = Command::new("ffmpeg");
        cmd.args([
            "-hide_banner", "-loglevel", "error", "-y", "-f", "rawvideo", "-pix_fmt",
            "bgr565le", "-s", "320x170", "-r", &fps, "-i", "-",
        ]);
        if !cfg.record.audio {
            cmd.arg("-an");
        }
        cmd.args([
            "-c:v", &cfg.record.encoder, "-b:v", &cfg.record.bitrate, "-pix_fmt",
            "yuv420p", "-movflags", "+faststart",
        ])
        .arg(&path);

        let logpath = dir.join("ffmpeg_rec.log");
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
        let handle = thread::spawn(move || {
            let mut frames = 1u64;
            loop {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                match capture::read_frame() {
                    Ok(raw) => {
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
            let status = child.wait();
            let mut s = st.lock().unwrap();
            s.recording = false;
            s.current_file = None;
            if let Ok(s2) = status {
                if !s2.success() {
                    s.last_error = Some(format!("ffmpeg exit {:?}", s2.code()));
                }
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
