// 守护进程 Unix socket 客户端

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::model::Item;

pub struct Client {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl Client {
    pub fn connect() -> Option<Client> {
        let sock = socket_path()?;
        let stream = UnixStream::connect(&sock).ok()?;
        let writer = stream.try_clone().ok()?;
        Some(Client {
            reader: BufReader::new(stream),
            writer,
        })
    }

    fn call(&mut self, cmd: &str, args: Value) -> Value {
        let mut payload = if args.is_object() { args } else { json!({}) };
        payload["cmd"] = json!(cmd);
        let line = serde_json::to_vec(&payload).unwrap_or_default();
        if self.writer.write_all(&line).is_err() || self.writer.write_all(b"\n").is_err() {
            return Value::Null;
        }
        let mut resp = String::new();
        if self.reader.read_line(&mut resp).is_err() {
            return Value::Null;
        }
        serde_json::from_str(&resp).unwrap_or(Value::Null)
    }

    pub fn status(&mut self) -> Option<Value> {
        self.call("status", json!({})).get("status").cloned()
    }

    pub fn config(&mut self) -> Option<Value> {
        self.call("config_get", json!({})).get("config").cloned()
    }

    pub fn items(&mut self) -> Vec<Item> {
        let v = self.call("items", json!({}));
        v.get("items")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| serde_json::from_value::<Item>(x.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn clear_all(&mut self) -> usize {
        let v = self.call("clear_all", json!({}));
        v.get("removed").and_then(|x| x.as_u64()).unwrap_or(0) as usize
    }

    pub fn thumb_path(&mut self, id: &str) -> Option<PathBuf> {
        let v = self.call("thumb", json!({ "id": id }));
        v.get("path")
            .and_then(|x| x.as_str())
            .map(PathBuf::from)
            .filter(|p| p.exists())
    }

    #[allow(dead_code)]
    pub fn shot(&mut self) -> bool {
        self.call("shot", json!({}))
            .get("ok")
            .and_then(|x| x.as_bool())
            .unwrap_or(false)
    }

    /// 返回录制状态：Some(true) 录制中，Some(false) 已停止，None 操作失败
    #[allow(dead_code)]
    pub fn toggle_record(&mut self) -> Option<bool> {
        self.call("record_toggle", json!({}))
            .get("recording")
            .and_then(|x| x.as_bool())
    }

    pub fn delete(&mut self, ids: &[String]) -> u64 {
        self.call("delete", json!({ "ids": ids }))
            .get("removed")
            .and_then(|x| x.as_u64())
            .unwrap_or(0)
    }

    pub fn config_set(&mut self, patch: Value) {
        self.call("config_set", json!({ "patch": patch }));
    }

    pub fn set_remote(&mut self, enabled: bool) {
        let cmd = if enabled { "remote_start" } else { "remote_stop" };
        self.call(cmd, json!({}));
    }
}

fn socket_path() -> Option<PathBuf> {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        let uid = unsafe { libc::getuid() };
        format!("/tmp/cardshot-{uid}")
    });
    Some(PathBuf::from(runtime).join("cardshot.sock"))
}
