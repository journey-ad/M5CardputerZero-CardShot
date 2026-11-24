use crate::{dispatch, Core};
use serde_json::json;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::sync::Arc;

#[repr(C)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

const UI_SET_EVBIT: libc::c_ulong = 0x40045564;
const UI_SET_KEYBIT: libc::c_ulong = 0x40045565;
const UI_DEV_CREATE: libc::c_ulong = 0x5501;
const UI_DEV_DESTROY: libc::c_ulong = 0x5502;
const EVIOCGRAB: libc::c_ulong = 0x40044590;
const EV_SYN: u16 = 0;
const EV_KEY: u16 = 1;

#[repr(C)]
struct UInputUserDev {
    name: [u8; 80],
    id: [u16; 4],
    ff_effects_max: u32,
    absmax: [i32; 64],
    absmin: [i32; 64],
    absfuzz: [i32; 64],
    absflat: [i32; 64],
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

/// 创建一张与物理键盘同键位的虚拟键盘，返回 (文件, 名称)
fn create_uinput() -> Option<(File, String)> {
    let mut f = OpenOptions::new()
        .write(true)
        .open("/dev/uinput")
        .map_err(|e| eprintln!("cardshotd: open /dev/uinput failed: {e}"))
        .ok()?;
    let fd = f.as_raw_fd();
    unsafe {
        libc::ioctl(fd, UI_SET_EVBIT, EV_KEY as libc::c_int);
        libc::ioctl(fd, UI_SET_EVBIT, EV_SYN as libc::c_int);
        for code in 1..=255 {
            libc::ioctl(fd, UI_SET_KEYBIT, code as libc::c_int);
        }
    }
    let mut dev = UInputUserDev {
        name: [0; 80],
        id: [0x03, 0x01, 0x01, 0x01],
        ff_effects_max: 0,
        absmax: [0; 64],
        absmin: [0; 64],
        absfuzz: [0; 64],
        absflat: [0; 64],
    };
    let name = b"cardshot virtual keyboard";
    dev.name[..name.len()].copy_from_slice(name);
    let bytes = unsafe {
        std::slice::from_raw_parts(&dev as *const UInputUserDev as *const u8, std::mem::size_of::<UInputUserDev>())
    };
    if f.write_all(bytes).is_err() {
        eprintln!("cardshotd: write uinput user dev failed");
        return None;
    }
    if unsafe { libc::ioctl(fd, UI_DEV_CREATE) } < 0 {
        eprintln!("cardshotd: UI_DEV_CREATE failed");
        return None;
    }
    Some((f, String::from_utf8_lossy(name).into_owned()))
}

pub fn spawn(core: Arc<Core>) {
    let dev = std::env::var("CARDSHOT_KBD")
        .ok()
        .or_else(|| std::env::var("APPLAUNCH_LINUX_KEYBOARD_DEVICE").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/dev/input/event3".into());
    let grab = core.cfg.lock().unwrap().hotkeys.grab;
    std::thread::spawn(move || {
        let mut f = match File::open(&dev) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("cardshotd: hotkeys open {dev} failed: {e}");
                return;
            }
        };
        // 开启拦截时：独占物理键盘，并把非热键事件转发到虚拟键盘
        let mut vk: Option<File> = None;
        if grab {
            if let Some((vf, name)) = create_uinput() {
                let r = unsafe { libc::ioctl(f.as_raw_fd(), EVIOCGRAB, 1 as libc::c_int) };
                if r == 0 {
                    eprintln!("cardshotd: hotkeys grabbed {dev}, forwarding via {name}");
                    vk = Some(vf);
                } else {
                    eprintln!("cardshotd: EVIOCGRAB {dev} failed ({r}), running passive");
                }
            } else {
                eprintln!("cardshotd: uinput unavailable, running passive");
            }
        }
        let mut buf = [0u8; std::mem::size_of::<InputEvent>()];
        let mut shift = false;
        let mut ctrl = false;
        let mut alt = false;
        // 被吞掉的按键，抬起时同步吞掉
        let mut swallowed: Vec<u16> = Vec::new();
        loop {
            if f.read_exact(&mut buf).is_err() {
                std::thread::sleep(std::time::Duration::from_millis(200));
                continue;
            }
            let ev: InputEvent = unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const InputEvent) };
            // 非按键事件（同步、扫描码）在有虚拟键盘时原样转发
            if ev.type_ != EV_KEY {
                if let Some(v) = vk.as_mut() {
                    let _ = v.write_all(&buf);
                }
                continue;
            }
            let modifier = matches!(ev.code, 42 | 54 | 29 | 97 | 56 | 100);
            if modifier {
                match ev.code {
                    42 | 54 => shift = ev.value != 0,
                    29 | 97 => ctrl = ev.value != 0,
                    56 | 100 => alt = ev.value != 0,
                    _ => {}
                }
                if let Some(v) = vk.as_mut() {
                    let _ = v.write_all(&buf);
                }
                continue;
            }
            if ev.value == 0 {
                if swallowed.contains(&ev.code) {
                    swallowed.retain(|c| *c != ev.code);
                    continue;
                }
                if let Some(v) = vk.as_mut() {
                    let _ = v.write_all(&buf);
                }
                continue;
            }
            if ev.value != 1 {
                // 自动重复：被吞键保持吞掉
                if swallowed.contains(&ev.code) {
                    continue;
                }
                if let Some(v) = vk.as_mut() {
                    let _ = v.write_all(&buf);
                }
                continue;
            }
            let cfg = core.cfg.lock().unwrap().clone();
            let mut handled = false;
            if cfg.hotkeys.enabled {
                let shot = parse_chord(&cfg.hotkeys.shot);
                let rec = parse_chord(&cfg.hotkeys.record);
                let matches = |ch: &Chord| {
                    ch.ctrl == ctrl && ch.alt == alt && ch.shift == shift && ch.code == Some(ev.code)
                };
                if matches(&shot) {
                    let _ = dispatch(&core, "shot", &json!({}));
                    handled = true;
                } else if matches(&rec) {
                    let _ = dispatch(&core, "record_toggle", &json!({}));
                    handled = true;
                }
            }
            if handled {
                swallowed.push(ev.code);
            } else if let Some(v) = vk.as_mut() {
                let _ = v.write_all(&buf);
            }
        }
    });
}
