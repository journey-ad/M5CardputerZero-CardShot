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
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("web")))
        .unwrap_or_else(|| PathBuf::from("web"))
}

pub fn display_host(bind: &str) -> String {
    if bind != "0.0.0.0" && bind != "::" {
        return bind.to_string();
    }
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

pub fn start(core: &Arc<Core>) -> Result<String, String> {
    if let Some(remote_handle) = core.remote.lock().unwrap().as_ref() {
        return Ok(remote_handle.url.clone());
    }
    let (bind, port) = {
        let config_guard = core.cfg.lock().unwrap();
        (config_guard.remote.bind.clone(), config_guard.remote.port)
    };
    let addr = format!("{}:{}", bind, port);
    let server = Server::http(&addr).map_err(|error| format!("bind {addr} failed: {error}"))?;
    let url = format!("http://{}:{}/", display_host(&bind), port);
    let stop = Arc::new(AtomicBool::new(false));
    let core_ref = core.clone();
    let stop_flag = stop.clone();
    let handle = std::thread::spawn(move || loop {
        if stop_flag.load(Ordering::SeqCst) {
            break;
        }
        match server.recv_timeout(Duration::from_millis(300)) {
            Ok(Some(req)) => handle_req(&core_ref, req),
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
    if let Some(mut remote_handle) = taken {
        remote_handle.stop.store(true, Ordering::SeqCst);
        if let Some(join_handle) = remote_handle.handle.take() {
            let _ = join_handle.join();
        }
    }
}

fn hdr(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).unwrap()
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

fn split_url(url: &str) -> (String, String) {
    match url.split_once('?') {
        Some((path, query)) => (path.to_string(), query.to_string()),
        None => (url.to_string(), String::new()),
    }
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn udecode(encoded: &str) -> String {
    let bytes = encoded.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2])) {
                out.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        if bytes[index] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[index]);
        }
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn qget(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let (name, value) = match pair.split_once('=') {
            Some(parts) => parts,
            None => (pair, ""),
        };
        if name == key {
            return Some(udecode(value));
        }
    }
    None
}

fn parse_range(range: &str, total: usize) -> Option<(usize, usize)> {
    let range = range.trim();
    let range = range.strip_prefix("bytes=")?;
    let (start_text, end_text) = range.split_once('-')?;
    let start = start_text.trim().parse::<usize>().ok()?;
    let end = if end_text.trim().is_empty() {
        total.saturating_sub(1)
    } else {
        end_text
            .trim()
            .parse::<usize>()
            .ok()?
            .min(total.saturating_sub(1))
    };
    if start > end || start >= total {
        return None;
    }
    Some((start, end))
}

fn send_json(req: tiny_http::Request, value: &Value) {
    let data = serde_json::to_vec(value).unwrap_or_default();
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
        let library = core.lib.lock().unwrap();
        library
            .items()
            .into_iter()
            .find(|item_entry| item_entry.id == id)
    };
    let path = match item {
        Some(item_entry) => core.lib.lock().unwrap().ensure_thumb(&item_entry),
        None => None,
    };
    match path {
        Some(path) => match std::fs::read(&path) {
            Ok(data) => {
                let _ = req.respond(
                    Response::from_data(data).with_header(hdr("Content-Type", "image/png")),
                );
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
        let library = core.lib.lock().unwrap();
        library.resolve(id).map(|(_, name, path)| (name, path))
    };
    let (name, path) = match resolved {
        Some(pair) => pair,
        None => {
            let _ = req.respond(Response::empty(StatusCode(404)));
            return;
        }
    };
    let data = match std::fs::read(&path) {
        Ok(data) => data,
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
        .find(|header| header.field.equiv("Range"))
        .map(|header| header.value.as_str().to_string());
    let mut status = 200u16;
    let mut slice: &[u8] = &data[..];
    let mut extra: Vec<Header> = Vec::new();
    if let Some(range) = range {
        if let Some((start_byte, end_byte)) = parse_range(&range, total) {
            status = 206;
            slice = &data[start_byte..=end_byte];
            extra.push(hdr(
                "Content-Range",
                &format!("bytes {}-{}/{}", start_byte, end_byte, total),
            ));
        }
    }
    let mut resp = Response::from_data(slice.to_vec())
        .with_status_code(StatusCode(status))
        .with_header(hdr("Content-Type", mime))
        .with_header(hdr("Accept-Ranges", "bytes"));
    for header in extra {
        resp = resp.with_header(header);
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
                let download_requested = qget(&query, "download").as_deref() == Some("1");
                return send_media(core, req, &id, download_requested);
            }
            _ => return send_static(req, &path),
        }
    }

    if method == Method::Post {
        let mut body = Vec::new();
        let _ = req.as_reader().read_to_end(&mut body);
        let value: Value = serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
        let resp = match path.as_str() {
            "/api/shot" => dispatch(core, "shot", &json!({})),
            "/api/record" => {
                let action = value
                    .get("action")
                    .and_then(|value| value.as_str())
                    .unwrap_or("toggle");
                let cmd = match action {
                    "start" => "record_start",
                    "stop" => "record_stop",
                    _ => "record_toggle",
                };
                dispatch(core, cmd, &json!({}))
            }
            "/api/delete" => dispatch(core, "delete", &value),
            "/api/config" => {
                let patch = value.get("patch").cloned().unwrap_or(value);
                core.apply_config(&patch)
            }
            "/api/remote" => {
                let enabled = value
                    .get("enabled")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                if enabled {
                    if let Some(port) = value.get("port").and_then(|value| value.as_u64()) {
                        let mut config_guard = core.cfg.lock().unwrap();
                        config_guard.remote.port = port as u16;
                        let _ = config_guard.save(&core.cfg_path);
                    }
                    match start(core) {
                        Ok(url) => json!({ "ok": true, "url": url }),
                        Err(error) => json!({ "ok": false, "error": error }),
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
