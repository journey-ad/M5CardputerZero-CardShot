use crate::{dispatch, Core};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::sync::Arc;

pub fn spawn(core: Arc<Core>, sock: String) {
    let _ = std::fs::remove_file(&sock);
    let listener = match UnixListener::bind(&sock) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("cardshotd: bind socket {sock} failed: {error}");
            return;
        }
    };
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = match stream {
                Ok(line) => line,
                Err(_) => continue,
            };
            let core = core.clone();
            std::thread::spawn(move || {
                let mut writer = stream.try_clone().ok();
                let reader = BufReader::new(stream);
                for line in reader.lines() {
                    let line = match line {
                        Ok(text_line) => text_line,
                        Err(_) => break,
                    };
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    let (command_name, args) = if line.starts_with('{') {
                        let value: Value =
                            serde_json::from_str(&line).unwrap_or_else(|_| json!({}));
                        let command = value
                            .get("cmd")
                            .and_then(|value| value.as_str())
                            .unwrap_or("")
                            .to_string();
                        (command, value)
                    } else {
                        (line, json!({}))
                    };
                    let resp = dispatch(&core, &command_name, &args);
                    if let Some(writer) = writer.as_mut() {
                        let _ = writer.write_all(&serde_json::to_vec(&resp).unwrap_or_default());
                        let _ = writer.write_all(b"\n");
                    }
                }
            });
        }
    });
}
