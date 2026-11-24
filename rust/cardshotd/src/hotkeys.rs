use crate::{dispatch, Core};
use serde_json::json;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

#[repr(C)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

struct Chord {
    shift: bool,
    code: Option<u16>,
}

fn parse_chord(s: &str) -> Chord {
    let l = s.to_lowercase();
    let shift = l.contains("shift");
    let code = if l.contains("space") {
        Some(57)
    } else if l.contains("enter") {
        Some(28)
    } else if l.contains("tab") {
        Some(15)
    } else {
        None
    };
    Chord { shift, code }
}

pub fn spawn(core: Arc<Core>) {
    let dev = std::env::var("APPLAUNCH_LINUX_KEYBOARD_DEVICE")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/dev/input/event3".into());
    std::thread::spawn(move || {
        let mut f = match File::open(&dev) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("cardshotd: hotkeys open {dev} failed: {e}");
                return;
            }
        };
        let mut buf = [0u8; std::mem::size_of::<InputEvent>()];
        let mut shift = false;
        loop {
            if f.read_exact(&mut buf).is_err() {
                std::thread::sleep(std::time::Duration::from_millis(200));
                continue;
            }
            let ev: InputEvent = unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const InputEvent) };
            if ev.type_ != 1 {
                continue;
            }
            if ev.code == 42 || ev.code == 54 {
                shift = ev.value != 0;
                continue;
            }
            if ev.value != 1 {
                continue;
            }
            let cfg = core.cfg.lock().unwrap().clone();
            let shot = parse_chord(&cfg.hotkeys.shot);
            let rec = parse_chord(&cfg.hotkeys.record);
            if shot.shift == shift && shot.code == Some(ev.code) {
                let _ = dispatch(&core, "shot", &json!({}));
            } else if rec.shift == shift && rec.code == Some(ev.code) {
                let _ = dispatch(&core, "record_toggle", &json!({}));
            }
        }
    });
}
