use crate::{dispatch, Core};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::sync::Arc;

pub fn spawn(core: Arc<Core>, sock: String) {
    let _ = std::fs::remove_file(&sock);
    let listener = match UnixListener::bind(&sock) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cardshotd: bind socket {sock} failed: {e}");
            return;
        }
    };
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            let core = core.clone();
            std::thread::spawn(move || {
                let mut writer = stream.try_clone().ok();
                let reader = BufReader::new(stream);
                for line in reader.lines() {
                    let line = match line {
                        Ok(l) => l,
                        Err(_) => break,
                    };
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    let (cmd, args) = if line.starts_with('{') {
                        let v: Value = serde_json::from_str(&line).unwrap_or_else(|_| json!({}));
                        let c = v
                            .get("cmd")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string();
                        (c, v)
                    } else {
                        (line, json!({}))
                    };
                    let resp = dispatch(&core, &cmd, &args);
                    if let Some(w) = writer.as_mut() {
                        let _ = w.write_all(&serde_json::to_vec(&resp).unwrap_or_default());
                        let _ = w.write_all(b"\n");
                    }
                }
            });
        }
    });
}
