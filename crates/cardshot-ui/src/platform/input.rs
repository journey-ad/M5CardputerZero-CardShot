// 键盘输入：读取 evdev 原始事件并按语义键名发送

use std::fs::File;
use std::io::Read;
use std::sync::mpsc;

use crate::constants::keycode;

pub const KEYBOARD_PATH: &str = "/dev/input/by-path/platform-3f804000.i2c-event";

fn code_name(code: u16) -> Option<&'static str> {
    Some(match code {
        keycode::UP => "UP",
        keycode::DOWN => "DOWN",
        keycode::LEFT => "LEFT",
        keycode::RIGHT => "RIGHT",
        keycode::ENTER => "ENTER",
        keycode::SPACE => "SPACE",
        keycode::ESC => "ESC",
        keycode::TAB => "TAB",
        keycode::BACKSPACE => "BACK",
        keycode::KEY_A => "A",
        keycode::KEY_B => "B",
        keycode::KEY_C => "C",
        keycode::KEY_D => "D",
        keycode::KEY_E => "E",
        keycode::KEY_F => "F",
        keycode::KEY_G => "G",
        keycode::KEY_H => "H",
        keycode::KEY_I => "I",
        keycode::KEY_J => "J",
        keycode::KEY_K => "K",
        keycode::KEY_L => "L",
        keycode::KEY_M => "M",
        keycode::KEY_N => "N",
        keycode::KEY_O => "O",
        keycode::KEY_P => "P",
        keycode::KEY_Q => "Q",
        keycode::KEY_R => "R",
        keycode::KEY_S => "S",
        keycode::KEY_T => "T",
        keycode::KEY_U => "U",
        keycode::KEY_V => "V",
        keycode::KEY_W => "W",
        keycode::KEY_X => "X",
        keycode::KEY_Y => "Y",
        keycode::KEY_Z => "Z",
        keycode::KEY_1 => "1",
        keycode::KEY_2 => "2",
        keycode::KEY_3 => "3",
        keycode::KEY_4 => "4",
        keycode::KEY_5 => "5",
        keycode::KEY_6 => "6",
        keycode::KEY_7 => "7",
        keycode::KEY_8 => "8",
        keycode::KEY_9 => "9",
        keycode::KEY_0 => "0",
        _ => return None,
    })
}

pub fn spawn_keys(path: String, tx: mpsc::Sender<String>) {
    std::thread::spawn(move || {
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(error) => {
                eprintln!("open keyboard {path} failed: {error}");
                return;
            }
        };
        // EV_KEY 结构：timeval(16) + type(2) + code(2) + value(4) = 24 字节
        let mut buf = [0u8; 24];
        let mut shift = false;
        let mut ctrl = false;
        let mut alt = false;
        loop {
            if file.read_exact(&mut buf).is_err() {
                break;
            }
            let event_type = u16::from_le_bytes([buf[16], buf[17]]);
            let code = u16::from_le_bytes([buf[18], buf[19]]);
            let value = i32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]);
            if event_type != 1 {
                continue;
            }
            // 修饰键的按下/抬起单独记录，供组合键与退格判断
            if code == keycode::SHIFT_LEFT || code == keycode::SHIFT_RIGHT {
                shift = value != 0;
                let _ = tx.send(if shift {
                    "SHIFT_ON".into()
                } else {
                    "SHIFT_OFF".into()
                });
                continue;
            }
            if code == keycode::LEFT_CTRL || code == keycode::RIGHT_CTRL {
                ctrl = value != 0;
                let _ = tx.send(if ctrl {
                    "CTRL_ON".into()
                } else {
                    "CTRL_OFF".into()
                });
                continue;
            }
            if code == keycode::LEFT_ALT || code == keycode::RIGHT_ALT {
                alt = value != 0;
                let _ = tx.send(if alt {
                    "ALT_ON".into()
                } else {
                    "ALT_OFF".into()
                });
                continue;
            }
            // ESC 抬起事件用于长按退出判断
            if code == keycode::ESC && value == 0 {
                let _ = tx.send("ESC_UP".to_string());
                continue;
            }
            if value != 1 {
                continue;
            }
            let name = if code == keycode::BACKSPACE {
                if shift {
                    "SHIFT_BACK"
                } else {
                    "BACK"
                }
            } else if code == keycode::TAB && shift {
                "SHIFT_TAB"
            } else {
                match code_name(code) {
                    Some(code) => code,
                    None => continue,
                }
            };
            if tx.send(name.to_string()).is_err() {
                break;
            }
        }
    });
}
