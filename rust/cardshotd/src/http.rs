use crate::{dispatch, Core};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tiny_http::{Header, Method, Response, Server, StatusCode};

pub struct RemoteHandle {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    pub url: String,
}

pub fn web_dir() -> PathBuf {
    if let Ok(d) = std::env::var("CARDSHOT_WEB_DIR") {
        if !d.is_empty() {
            return PathBuf::from(d);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            let w = p.join("web");
            if w.is_dir() {
                return w;
            }
        }
    }
    PathBuf::from("web")
}

fn display_host(bind: &str) -> String {
    if bind != "0.0.0.0" && bind != "::" {
        return bind.to_string();
    }
    if let Ok(s) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if s.connect("8.8.8.8:80").is_ok() {
            if let Ok(a) = s.local_addr() {
                return a.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

pub fn start(core: &Arc<Core>) -> Result<String, String> {
    if let Some(h) = core.remote.lock().unwrap().as_ref() {
        return Ok(h.url.clone());
    }
    let (bind, port) = {
        let c = core.cfg.lock().unwrap();
        (c.remote.bind.clone(), c.remote.port)
    };
    let addr = format!("{}:{}", bind, port);
    let server = Server::http(&addr).map_err(|e| format!("bind {addr} failed: {e}"))?;
    let url = format!("http://{}:{}/", display_host(&bind), port);
    let stop = Arc::new(AtomicBool::new(false));
    let c = core.clone();
    let st = stop.clone();
    let handle = std::thread::spawn(move || loop {
        if st.load(Ordering::SeqCst) {
            break;
        }
        match server.recv_timeout(Duration::from_millis(300)) {
            Ok(Some(req)) => handle_req(&c, req),
            Ok(None) => {}
            Err(_) => break,
        }
    });
    core.remote.lock().unwrap().replace(RemoteHandle {
        stop,
        handle: Some(handle),
        url: url.clone(),
    });
    Ok(url)
}

pub fn stop(core: &Arc<Core>) {
    let taken = core.remote.lock().unwrap().take();
    if let Some(mut h) = taken {
        h.stop.store(true, Ordering::SeqCst);
        if let Some(j) = h.handle.take() {
            let _ = j.join();
        }
    }
}

fn hdr(k: &str, v: &str) -> Header {
    Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap()
}

fn mime_for(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}

fn split_url(u: &str) -> (String, String) {
    match u.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (u.to_string(), String::new()),
    }
}

fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn udecode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        if b[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(b[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn qget(q: &str, key: &str) -> Option<String> {
    for pair in q.split('&') {
        let (k, v) = match pair.split_once('=') {
            Some(x) => x,
            None => (pair, ""),
        };
        if k == key {
            return Some(udecode(v));
        }
    }
    None
}

fn parse_range(r: &str, total: usize) -> Option<(usize, usize)> {
    let r = r.trim();
    let r = r.strip_prefix("bytes=")?;
    let (a, b) = r.split_once('-')?;
    let start = a.trim().parse::<usize>().ok()?;
    let end = if b.trim().is_empty() {
        total.saturating_sub(1)
    } else {
        b.trim().parse::<usize>().ok()?.min(total.saturating_sub(1))
    };
    if start > end || start >= total {
        return None;
    }
    Some((start, end))
}

fn send_json(req: tiny_http::Request, v: &Value) {
    let data = serde_json::to_vec(v).unwrap_or_default();
    let _ = req.respond(
        Response::from_data(data)
            .with_header(hdr("Content-Type", "application/json"))
            .with_header(hdr("Cache-Control", "no-store")),
    );
}

fn send_file(req: tiny_http::Request, path: &Path) {
    match std::fs::read(path) {
        Ok(data) => {
            let mime = mime_for(&path.to_string_lossy());
            let _ = req.respond(Response::from_data(data).with_header(hdr("Content-Type", mime)));
        }
        Err(_) => {
            let _ = req.respond(Response::empty(StatusCode(404)));
        }
    }
}

fn send_static(req: tiny_http::Request, path: &str) {
    let rel = path.trim_start_matches('/');
    if rel.contains("..") {
        let _ = req.respond(Response::empty(StatusCode(404)));
        return;
    }
    let rel = if rel.is_empty() { "index.html" } else { rel };
    send_file(req, &web_dir().join(rel));
}

fn send_thumb(core: &Arc<Core>, req: tiny_http::Request, id: &str) {
    let item = {
        let lib = core.lib.lock().unwrap();
        lib.items().into_iter().find(|i| i.id == id)
    };
    let path = match item {
        Some(it) => core.lib.lock().unwrap().ensure_thumb(&it),
        None => None,
    };
    match path {
        Some(p) => match std::fs::read(&p) {
            Ok(d) => {
                let _ = req.respond(Response::from_data(d).with_header(hdr("Content-Type", "image/png")));
            }
            Err(_) => {
                let _ = req.respond(Response::empty(StatusCode(404)));
            }
        },
        None => {
            let _ = req.respond(Response::empty(StatusCode(404)));
        }
    }
}

fn send_media(core: &Arc<Core>, req: tiny_http::Request, id: &str, download: bool) {
    let resolved = {
        let lib = core.lib.lock().unwrap();
        lib.resolve(id).map(|(_, name, p)| (name, p))
    };
    let (name, path) = match resolved {
        Some(x) => x,
        None => {
            let _ = req.respond(Response::empty(StatusCode(404)));
            return;
        }
    };
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            let _ = req.respond(Response::empty(StatusCode(404)));
            return;
        }
    };
    let total = data.len();
    let mime = mime_for(&name);
    let range = req
        .headers()
        .iter()
        .find(|h| h.field.equiv("Range"))
        .map(|h| h.value.as_str().to_string());
    let mut status = 200u16;
    let mut slice: &[u8] = &data[..];
    let mut extra: Vec<Header> = Vec::new();
    if let Some(r) = range {
        if let Some((s, e)) = parse_range(&r, total) {
            status = 206;
            slice = &data[s..=e];
            extra.push(hdr("Content-Range", &format!("bytes {}-{}/{}", s, e, total)));
        }
    }
    let mut resp = Response::from_data(slice.to_vec())
        .with_status_code(StatusCode(status))
        .with_header(hdr("Content-Type", mime))
        .with_header(hdr("Accept-Ranges", "bytes"));
    for h in extra {
        resp = resp.with_header(h);
    }
    if download {
        resp = resp.with_header(hdr(
            "Content-Disposition",
            &format!("attachment; filename=\"{}\"", name),
        ));
    }
    let _ = req.respond(resp);
}

fn handle_req(core: &Arc<Core>, mut req: tiny_http::Request) {
    let method = req.method().clone();
    let url = req.url().to_string();
    let (path, query) = split_url(&url);

    if method == Method::Get {
        match path.as_str() {
            "/" | "/index.html" => return send_file(req, &web_dir().join("index.html")),
            "/api/status" => return send_json(req, &dispatch(core, "status", &json!({}))),
            "/api/items" => return send_json(req, &dispatch(core, "items", &json!({}))),
            "/api/config" => return send_json(req, &dispatch(core, "config_get", &json!({}))),
            "/api/thumb" => {
                let id = qget(&query, "id").unwrap_or_default();
                return send_thumb(core, req, &id);
            }
            "/api/file" => {
                let id = qget(&query, "id").unwrap_or_default();
                let dl = qget(&query, "download").as_deref() == Some("1");
                return send_media(core, req, &id, dl);
            }
            _ => return send_static(req, &path),
        }
    }

    if method == Method::Post {
        let mut body = Vec::new();
        let _ = req.as_reader().read_to_end(&mut body);
        let v: Value = serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
        let resp = match path.as_str() {
            "/api/shot" => dispatch(core, "shot", &json!({})),
            "/api/record" => {
                let action = v.get("action").and_then(|x| x.as_str()).unwrap_or("toggle");
                let cmd = match action {
                    "start" => "record_start",
                    "stop" => "record_stop",
                    _ => "record_toggle",
                };
                dispatch(core, cmd, &json!({}))
            }
            "/api/delete" => dispatch(core, "delete", &v),
            "/api/config" => {
                let patch = v.get("patch").cloned().unwrap_or(v);
                core.apply_config(&patch)
            }
            "/api/remote" => {
                let enabled = v.get("enabled").and_then(|x| x.as_bool()).unwrap_or(false);
                if enabled {
                    if let Some(port) = v.get("port").and_then(|x| x.as_u64()) {
                        let mut c = core.cfg.lock().unwrap();
                        c.remote.port = port as u16;
                        let _ = c.save(&core.cfg_path);
                    }
                    match start(core) {
                        Ok(u) => json!({ "ok": true, "url": u }),
                        Err(e) => json!({ "ok": false, "error": e }),
                    }
                } else {
                    stop(core);
                    json!({ "ok": true })
                }
            }
            _ => json!({ "ok": false, "error": "not found" }),
        };
        return send_json(req, &resp);
    }

    let _ = req.respond(Response::empty(StatusCode(405)));
}
