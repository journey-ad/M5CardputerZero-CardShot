mod control;
mod feedback;
mod hotkeys;
mod http;

use cardshot_core::overlay;
use cardshot_core::config::Config;
use cardshot_core::library::Library;
use cardshot_core::recorder::Recorder;
use cardshot_core::capture;
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
    let mut v = serde_json::to_value(cfg).unwrap_or_else(|_| json!({}));
    if let Some(o) = v.as_object_mut() {
        let mut remote = o.get("remote").cloned().unwrap_or_else(|| json!({}));
        if let Some(ro) = remote.as_object_mut() {
            ro.insert("running".to_string(), json!(remote_running));
        }
        o.insert("remote".to_string(), remote);
    }
    v
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
        let (im, vi, tb) = self.lib.lock().unwrap().stats();
        let rec = self.recorder.lock().unwrap().status();
        let running = self.remote.lock().unwrap().is_some();
        let cfg = self.cfg.lock().unwrap();
        let mut v = json!({
            "recording": rec.recording,
            "elapsed": rec.elapsed,
            "frames": rec.frames,
            "current_file": rec.current_file,
            "last_error": rec.last_error,
            "counts": { "images": im, "videos": vi },
            "total_bytes": tb,
            "remote": { "running": running, "port": cfg.remote.port, "bind": cfg.remote.bind.clone(), "host": crate::http::display_host(&cfg.remote.bind) },
            "output_dir": cfg.output_dir.clone(),
            "video_dir": cfg.video_dir.clone(),
            "image_format": cfg.image_format.clone(),
            "image_quality": cfg.image_quality,
        });
        let _ = &mut v;
        v
    }

    pub fn public_config(&self) -> Value {
        let cfg = self.cfg.lock().unwrap();
        let running = self.remote.lock().unwrap().is_some();
        config_json(&cfg, running)
    }

    pub fn apply_config(&self, patch: &Value) -> Value {
        let mut cfg = self.cfg.lock().unwrap();
        let mut cur = serde_json::to_value(&*cfg).unwrap_or_else(|_| json!({}));
        merge(&mut cur, patch);
        match serde_json::from_value::<Config>(cur) {
            Ok(mut nc) => {
                nc.image_quality = nc.image_quality.clamp(1, 100);
                if !matches!(nc.language.as_str(), "zh" | "en" | "ja") {
                    nc.language = "zh".into();
                }
                let old_autostart = cfg.service.autostart;
                let _ = nc.save(&self.cfg_path);
                if old_autostart != nc.service.autostart {
                    let action = if nc.service.autostart { "enable" } else { "disable" };
                    let rt = crate::runtime_dir();
                    let _ = std::process::Command::new("systemctl")
                        .args(["--user", action, "cardshot.service"])
                        .env("XDG_RUNTIME_DIR", &rt)
                        .status();
                }
                *cfg = nc.clone();
                *self.lib.lock().unwrap() = Library::new(&nc);
                let running = self.remote.lock().unwrap().is_some();
                json!({ "ok": true, "config": config_json(&nc, running) })
            }
            Err(e) => json!({ "ok": false, "error": e.to_string() }),
        }
    }

    pub fn reload_config(&self) {
        let cfg = Config::load(&self.cfg_path);
        let lib = Library::new(&cfg);
        *self.cfg.lock().unwrap() = cfg;
        *self.lib.lock().unwrap() = lib;
    }
}

fn merge(dst: &mut Value, src: &Value) {
    if let (Some(d), Some(s)) = (dst.as_object_mut(), src.as_object()) {
        for (k, v) in s {
            match d.get_mut(k) {
                Some(dv) if dv.is_object() && v.is_object() => merge(dv, v),
                Some(dv) => *dv = v.clone(),
                None => {
                    d.insert(k.clone(), v.clone());
                }
            }
        }
    } else {
        *dst = src.clone();
    }
}

pub fn dispatch(core: &Arc<Core>, cmd: &str, args: &Value) -> Value {
    match cmd {
        "ping" => json!({ "ok": true, "pong": true }),
        "status" => json!({ "ok": true, "status": core.status() }),
        "shot" => {
            let cfg = core.cfg.lock().unwrap();
            match capture::screenshot(&cfg.output_dir, &cfg.image_format, cfg.image_quality) {
                Ok(p) => {
                    drop(cfg);
                    core.feedback.flash();
                    let fb = capture::fb_path();
                    std::thread::spawn(move || { let _ = overlay::toast(&fb, include_bytes!("../assets/shot.png")); });
                    json!({ "ok": true, "path": p.to_string_lossy() })
                }
                Err(e) => json!({ "ok": false, "error": e.to_string() }),
            }
        }
        "record_start" => {
            let cfg = core.cfg.lock().unwrap().clone();
            let _ = overlay::toast(&capture::fb_path(), include_bytes!("../assets/rec_start.png"));
            let mut r = core.recorder.lock().unwrap();
            match r.start(&cfg) {
                Ok(p) => {
                    drop(r);
                    core.feedback.flash();
                    json!({ "ok": true, "recording": true, "path": p.to_string_lossy() })
                }
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
        "record_stop" => {
            let mut r = core.recorder.lock().unwrap();
            match r.stop() {
                Ok(p) => {
                    drop(r);
                    core.feedback.flash();
                    let fb = capture::fb_path();
                    std::thread::spawn(move || { let _ = overlay::toast(&fb, include_bytes!("../assets/rec_saved.png")); });
                    json!({ "ok": true, "recording": false, "path": p.to_string_lossy() })
                }
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
        "record_toggle" => {
            let cfg = core.cfg.lock().unwrap().clone();
            let mut r = core.recorder.lock().unwrap();
            let was = r.status().recording;
            if !was {
                let _ = overlay::toast(&capture::fb_path(), include_bytes!("../assets/rec_start.png"));
            }
            match r.toggle(&cfg) {
                Ok((on, p)) => {
                    drop(r);
                    core.feedback.flash();
                    if was && !on {
                        let fb = capture::fb_path();
                        std::thread::spawn(move || { let _ = overlay::toast(&fb, include_bytes!("../assets/rec_saved.png")); });
                    }
                    json!({ "ok": true, "recording": on, "path": p.to_string_lossy() })
                }
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
        "items" => {
            let items = core.lib.lock().unwrap().items();
            json!({ "ok": true, "items": items })
        }
        "thumb" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let lib = core.lib.lock().unwrap();
            let found = lib.items().into_iter().find(|i| i.id == id);
            match found {
                Some(it) => match lib.ensure_thumb(&it) {
                    Some(p) => json!({ "ok": true, "path": p.to_string_lossy() }),
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
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let n = core.lib.lock().unwrap().delete(&ids);
            json!({ "ok": true, "removed": n })
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
            let n = core.lib.lock().unwrap().delete(&ids);
            json!({ "ok": true, "removed": n })
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
        "remote_start" => {
            match http::start(core) {
                Ok(url) => {
                    {
                        let mut cfg = core.cfg.lock().unwrap();
                        cfg.remote.enabled = true;
                        let _ = cfg.save(&core.cfg_path);
                    }
                    json!({ "ok": true, "url": url })
                }
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
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
            let running = core.remote.lock().unwrap().as_ref().map(|h| h.url.clone());
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
    if let Ok(d) = std::env::var("XDG_RUNTIME_DIR") {
        if !d.is_empty() {
            return PathBuf::from(d);
        }
    }
    format!("/tmp/cardshot-{}", unsafe { libc::getuid() }).into()
}

fn acquire_lock(path: &Path) -> bool {
    use std::os::unix::io::AsRawFd;
    let f = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)
    {
        Ok(f) => f,
        Err(_) => return false,
    };
    let rc = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc != 0 {
        return false;
    }
    std::mem::forget(f);
    true
}

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_sig: libc::c_int) {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

fn main() {
    let mut cfg_path = default_cfg_path();
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        if a == "--config" {
            if let Some(v) = it.next() {
                cfg_path = v;
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
            Ok(u) => eprintln!("cardshotd: remote web {u}"),
            Err(e) => eprintln!("cardshotd: remote start failed: {e}"),
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
        let mut r = core.recorder.lock().unwrap();
        if r.is_recording() {
            let _ = r.stop();
        }
    }
    http::stop(&core);
    let _ = std::fs::remove_file(&sock);
    eprintln!("cardshotd: exit");
}
