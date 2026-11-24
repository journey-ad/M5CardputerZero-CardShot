pub struct Feedback {
    pub enabled: bool,
}

impl Feedback {
    pub fn new(enabled: bool) -> Self {
        Feedback { enabled }
    }

    pub fn flash(&self) {
        if !self.enabled {
            return;
        }
        let path = "/sys/class/leds/ACT/brightness";
        if !std::path::Path::new(path).exists() {
            return;
        }
        std::thread::spawn(move || {
            for _ in 0..2 {
                let _ = std::fs::write(path, b"1");
                std::thread::sleep(std::time::Duration::from_millis(80));
                let _ = std::fs::write(path, b"0");
                std::thread::sleep(std::time::Duration::from_millis(80));
            }
        });
    }
}
