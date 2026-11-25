mod control;
mod feedback;
mod hotkeys;
mod http;

use cardshot_core::capture;
use cardshot_core::config::Config;
use cardshot_core::library::Library;
use cardshot_core::overlay;
use cardshot_core::recorder::Recorder;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct Core {
    pub cfg_path: String,
    pub cfg: Mutex<Config>,
    pub lib: Mutex<Library>,
    pub recorder: Mutex<Recorder>,
    pub feedback: feedback::Feedback,
    pub remote: Mutex<Option<http::RemoteHandle>>,
}

fn config_json(cfg: &Config, remote_running: bool) -> Value {
    let mut json_value = serde_json::to_value(cfg).unwrap_or_else(|_| json!({}));
    if let Some(object_map) = json_value.as_object_mut() {
        let mut remote_value = object_map
            .get("remote")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if let Some(remote_map) = remote_value.as_object_mut() {
            remote_map.insert("running".to_string(), json!(remote_running));
        }
        object_map.insert("remote".to_string(), remote_value);
    }
    json_value
}

impl Core {
    pub fn new(cfg_path: String, cfg: Config) -> Core {
        let lib = Library::new(&cfg);
        let fb = feedback::Feedback::new(cfg.feedback.led);
        Core {
            cfg_path,
            cfg: Mutex::new(cfg),
            lib: Mutex::new(lib),
            recorder: Mutex::new(Recorder::new()),
            feedback: fb,
            remote: Mutex::new(None),
        }
    }

    pub fn status(&self) -> Value {
        let (image_count, video_count, total_bytes) = self.lib.lock().unwrap().stats();
        let record_status = self.recorder.lock().unwrap().status();
        let remote_running = self.remote.lock().unwrap().is_some();
        let config = self.cfg.lock().unwrap();
        let mut status_json = json!({
            "recording": record_status.recording,
            "elapsed": record_status.elapsed,
            "frames": record_status.frames,
            "current_file": record_status.current_file,
            "last_error": record_status.last_error,
            "counts": { "images": image_count, "videos": video_count },
            "total_bytes": total_bytes,
            "remote": { "running": remote_running, "port": config.remote.port, "bind": config.remote.bind.clone(), "host": crate::http::display_host(&config.remote.bind) },
            "output_dir": config.output_dir.clone(),
            "video_dir": config.video_dir.clone(),
            "image_format": config.image_format.clone(),
            "image_quality": config.image_quality,
        });
        status_json
    }

    pub fn public_config(&self) -> Value {
        let config = self.cfg.lock().unwrap();
        let remote_running = self.remote.lock().unwrap().is_some();
        config_json(&config, remote_running)
    }

    pub fn apply_config(&self, patch: &Value) -> Value {
        let mut config = self.cfg.lock().unwrap();
        let mut merged = serde_json::to_value(&*config).unwrap_or_else(|_| json!({}));
        merge(&mut merged, patch);
        match serde_json::from_value::<Config>(merged) {
            Ok(mut new_config) => {
                new_config.image_quality = new_config.image_quality.clamp(1, 100);
                if !matches!(new_config.language.as_str(), "zh" | "en" | "ja") {
                    new_config.language = "en".into();
                }
                let old_autostart = config.service.autostart;
                let _ = new_config.save(&self.cfg_path);
                if old_autostart != new_config.service.autostart {
                    let action = if new_config.service.autostart {
                        "enable"
                    } else {
                        "disable"
                    };
                    let runtime_path = crate::runtime_dir();
                    let _ = std::process::Command::new("systemctl")
                        .args(["--user", action, "cardshot.service"])
                        .env("XDG_RUNTIME_DIR", &runtime_path)
                        .status();
                }
                *config = new_config.clone();
                self.lib.lock().unwrap().reconfigure(&new_config);
                let remote_running = self.remote.lock().unwrap().is_some();
                json!({ "ok": true, "config": config_json(&new_config, remote_running) })
            }
            Err(error) => json!({ "ok": false, "error": error.to_string() }),
        }
    }

    pub fn reload_config(&self) {
        let config = Config::load(&self.cfg_path);
        let library = Library::new(&config);
        *self.cfg.lock().unwrap() = config;
        *self.lib.lock().unwrap() = library;
    }
}

fn merge(dst: &mut Value, src: &Value) {
    if let (Some(dst_map), Some(src_map)) = (dst.as_object_mut(), src.as_object()) {
        for (key, value) in src_map {
            match dst_map.get_mut(key) {
                Some(dst_value) if dst_value.is_object() && value.is_object() => {
                    merge(dst_value, value)
                }
                Some(dst_value) => *dst_value = value.clone(),
                None => {
                    dst_map.insert(key.clone(), value.clone());
                }
            }
        }
    } else {
        *dst = src.clone();
    }
}

pub fn dispatch(core: &Arc<Core>, command_name: &str, args: &Value) -> Value {
    match command_name {
        "ping" => json!({ "ok": true, "pong": true }),
        "status" => json!({ "ok": true, "status": core.status() }),
        "shot" => {
            let cfg = core.cfg.lock().unwrap();
            match capture::screenshot(&cfg.output_dir, &cfg.image_format, cfg.image_quality) {
                Ok(path) => {
                    drop(cfg);
                    core.feedback.flash();
                    let fb = capture::FB_PATH;
                    std::thread::spawn(move || {
                        let _ = overlay::toast(&fb, include_bytes!("../assets/shot.png"));
                    });
                    json!({ "ok": true, "path": path.to_string_lossy() })
                }
                Err(error) => json!({ "ok": false, "error": error.to_string() }),
            }
        }
        "record_start" => {
            let cfg = core.cfg.lock().unwrap().clone();
            let _ = overlay::toast(&capture::FB_PATH, include_bytes!("../assets/rec_start.png"));
            let mut recorder = core.recorder.lock().unwrap();
            match recorder.start(&cfg) {
                Ok(path) => {
                    drop(recorder);
                    core.feedback.flash();
                    json!({ "ok": true, "recording": true, "path": path.to_string_lossy() })
                }
                Err(error) => json!({ "ok": false, "error": error }),
            }
        }
        "record_stop" => {
            let mut recorder = core.recorder.lock().unwrap();
            match recorder.stop() {
                Ok(path) => {
                    drop(recorder);
                    core.feedback.flash();
                    let fb = capture::FB_PATH;
                    std::thread::spawn(move || {
                        let _ = overlay::toast(&fb, include_bytes!("../assets/rec_saved.png"));
                    });
                    json!({ "ok": true, "recording": false, "path": path.to_string_lossy() })
                }
                Err(error) => json!({ "ok": false, "error": error }),
            }
        }
        "record_toggle" => {
            let cfg = core.cfg.lock().unwrap().clone();
            let mut recorder = core.recorder.lock().unwrap();
            let was = recorder.status().recording;
            if !was {
                let _ =
                    overlay::toast(&capture::FB_PATH, include_bytes!("../assets/rec_start.png"));
            }
            match recorder.toggle(&cfg) {
                Ok((on, path)) => {
                    drop(recorder);
                    core.feedback.flash();
                    if was && !on {
                        let fb = capture::FB_PATH;
                        std::thread::spawn(move || {
                            let _ = overlay::toast(&fb, include_bytes!("../assets/rec_saved.png"));
                        });
                    }
                    json!({ "ok": true, "recording": on, "path": path.to_string_lossy() })
                }
                Err(error) => json!({ "ok": false, "error": error }),
            }
        }
        "items" => {
            let items = core.lib.lock().unwrap().items();
            json!({ "ok": true, "items": items })
        }
        "thumb" => {
            let id = args
                .get("id")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let lib = core.lib.lock().unwrap();
            let found = lib.items().into_iter().find(|item| item.id == id);
            match found {
                Some(it) => match lib.ensure_thumb(&it) {
                    Some(path) => json!({ "ok": true, "path": path.to_string_lossy() }),
                    None => json!({ "ok": false, "error": "thumb failed" }),
                },
                None => json!({ "ok": false, "error": "not found" }),
            }
        }
        "counts" => {
            let (im, vi, tb) = core.lib.lock().unwrap().stats();
            json!({ "ok": true, "images": im, "videos": vi, "total_bytes": tb })
        }
        "delete" => {
            let ids: Vec<String> = args
                .get("ids")
                .and_then(|value| value.as_array())
                .map(|array| {
                    array
                        .iter()
                        .filter_map(|value| value.as_str().map(|src| src.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let removed = core.lib.lock().unwrap().delete(&ids);
            json!({ "ok": true, "removed": removed })
        }
        "clear_all" => {
            let ids: Vec<String> = core
                .lib
                .lock()
                .unwrap()
                .items()
                .iter()
                .map(|it| it.id.clone())
                .collect();
            let removed = core.lib.lock().unwrap().delete(&ids);
            json!({ "ok": true, "removed": removed })
        }
        "config_get" => json!({ "ok": true, "config": core.public_config() }),
        "config_set" => {
            let patch = args.get("patch").cloned().unwrap_or_else(|| args.clone());
            core.apply_config(&patch)
        }
        "reload" => {
            core.reload_config();
            json!({ "ok": true })
        }
        "remote_start" => match http::start(core) {
            Ok(url) => {
                {
                    let mut cfg = core.cfg.lock().unwrap();
                    cfg.remote.enabled = true;
                    let _ = cfg.save(&core.cfg_path);
                }
                json!({ "ok": true, "url": url })
            }
            Err(error) => json!({ "ok": false, "error": error }),
        },
        "remote_stop" => {
            http::stop(core);
            {
                let mut cfg = core.cfg.lock().unwrap();
                cfg.remote.enabled = false;
                let _ = cfg.save(&core.cfg_path);
            }
            json!({ "ok": true })
        }
        "remote_status" => {
            let running = core
                .remote
                .lock()
                .unwrap()
                .as_ref()
                .map(|handle| handle.url.clone());
            match running {
                Some(url) => json!({ "ok": true, "running": true, "url": url }),
                None => json!({ "ok": true, "running": false }),
            }
        }
        other => json!({ "ok": false, "error": format!("unknown command: {}", other) }),
    }
}

fn default_cfg_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    format!("{home}/.config/cardshot/config.json")
}

fn runtime_dir() -> PathBuf {
    if let Ok(dst) = std::env::var("XDG_RUNTIME_DIR") {
        if !dst.is_empty() {
            return PathBuf::from(dst);
        }
    }
    format!("/tmp/cardshot-{}", unsafe { libc::getuid() }).into()
}

fn acquire_lock(path: &Path) -> bool {
    use std::os::unix::io::AsRawFd;
    let file = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)
    {
        Ok(file) => file,
        Err(_) => return false,
    };
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc != 0 {
        return false;
    }
    std::mem::forget(file);
    true
}

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_sig: libc::c_int) {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

fn main() {
    let mut cfg_path = default_cfg_path();
    let mut it = std::env::args().skip(1);
    while let Some(array) = it.next() {
        if array == "--config" {
            if let Some(path_value) = it.next() {
                cfg_path = path_value;
            }
        }
    }

    let cfg = Config::load(&cfg_path);
    let core = Arc::new(Core::new(cfg_path.clone(), cfg.clone()));

    let rt = runtime_dir();
    std::fs::create_dir_all(&rt).ok();
    let lock_path = rt.join("cardshot.lock");
    if !acquire_lock(&lock_path) {
        eprintln!("cardshotd: another instance is already running");
        std::process::exit(1);
    }

    let sock = rt.join(&cfg.socket_name).to_string_lossy().into_owned();
    control::spawn(core.clone(), sock.clone());
    eprintln!("cardshotd: control socket {sock}");

    if cfg.hotkeys.enabled {
        hotkeys::spawn(core.clone());
        eprintln!("cardshotd: hotkeys enabled");
    }

    if cfg.remote.enabled {
        match http::start(&core) {
            Ok(url) => eprintln!("cardshotd: remote web {url}"),
            Err(error) => eprintln!("cardshotd: remote start failed: {error}"),
        }
    }

    unsafe {
        libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
    }

    while !SHUTDOWN.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    // 退出前停止录制与远程服务
    {
        let mut recorder = core.recorder.lock().unwrap();
        if recorder.is_recording() {
            let _ = recorder.stop();
        }
    }
    http::stop(&core);
    let _ = std::fs::remove_file(&sock);
    eprintln!("cardshotd: exit");
}
