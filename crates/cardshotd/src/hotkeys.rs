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

/// 一个组合键：修饰键状态 + 主键键码
struct Chord {
    ctrl: bool,
    alt: bool,
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
    let mut chord = Chord { ctrl: false, alt: false, shift: false, code: None };
    for part in l.split('+') {
        match part.trim() {
            "ctrl" | "control" => chord.ctrl = true,
            "alt" => chord.alt = true,
            "shift" => chord.shift = true,
            other => {
                if !other.is_empty() {
                    chord.code = key_code(other);
                }
            }
        }
    }
    chord
}

pub fn spawn(core: Arc<Core>) {
    let dev = "/dev/input/by-path/platform-3f804000.i2c-event";
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
        let mut ctrl = false;
        let mut alt = false;
        loop {
            if f.read_exact(&mut buf).is_err() {
                std::thread::sleep(std::time::Duration::from_millis(200));
                continue;
            }
            let ev: InputEvent = unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const InputEvent) };
            if ev.type_ != 1 {
                continue;
            }
            match ev.code {
                42 | 54 => {
                    shift = ev.value != 0;
                    continue;
                }
                29 | 97 => {
                    ctrl = ev.value != 0;
                    continue;
                }
                56 | 100 => {
                    alt = ev.value != 0;
                    continue;
                }
                _ => {}
            }
            if ev.value != 1 {
                continue;
            }
            let cfg = core.cfg.lock().unwrap().clone();
            if !cfg.hotkeys.enabled {
                continue;
            }
            let shot = parse_chord(&cfg.hotkeys.shot);
            let rec = parse_chord(&cfg.hotkeys.record);
            let matches = |ch: &Chord| ch.ctrl == ctrl && ch.alt == alt && ch.shift == shift && ch.code == Some(ev.code);
            if matches(&shot) {
                let _ = dispatch(&core, "shot", &json!({}));
            } else if matches(&rec) {
                let _ = dispatch(&core, "record_toggle", &json!({}));
            }
        }
    });
}
