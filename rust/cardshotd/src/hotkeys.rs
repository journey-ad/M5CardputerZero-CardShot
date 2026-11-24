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

fn key_code(name: &str) -> Option<u16> {
    Some(match name {
        "space" => 57,
        "enter" => 28,
        "tab" => 15,
        "a" => 30, "b" => 48, "c" => 46, "d" => 32, "e" => 18, "f" => 33,
        "g" => 34, "h" => 35, "i" => 23, "j" => 36, "k" => 37, "l" => 38,
        "m" => 50, "n" => 49, "o" => 24, "p" => 25, "q" => 16, "r" => 19,
        "s" => 31, "t" => 20, "u" => 22, "v" => 47, "w" => 17, "x" => 45,
        "y" => 21, "z" => 44,
        "1" => 2, "2" => 3, "3" => 4, "4" => 5, "5" => 6,
        "6" => 7, "7" => 8, "8" => 9, "9" => 10, "0" => 11,
        _ => return None,
    })
}

fn parse_chord(s: &str) -> Chord {
    let l = s.to_lowercase();
    let shift = l.contains("shift");
    // 取最后一个 + 分隔段作为主键
    let key = l.rsplit('+').next().unwrap_or("").trim();
    Chord { shift, code: key_code(key) }
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
