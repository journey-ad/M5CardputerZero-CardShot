// CardShot 设备端界面入口：平台初始化与主循环

mod app;
mod constants;
mod daemon;
mod fonts;
mod model;
mod platform;

slint::include_modules!();

use std::sync::mpsc;
use std::time::{Duration, Instant};

use slint::ComponentHandle;

use app::App;
use constants::{FRAME_SLEEP_MS, POLL_INTERVAL_MS};
use platform::input;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut display = platform::Display::new()?;

    let ui = AppWindow::new()?;
    ui.show()?;

    let (tx, rx) = mpsc::channel::<String>();
    input::spawn_keys(input::keyboard_path(), tx);

    let mut application = App::new(ui);
    application.init();
    // 调试钩子：启动后注入一串按键，便于离线验证交互
    if let Ok(keys) = std::env::var("CARDSHOT_TEST_KEYS") {
        for k in keys.split(',').filter(|s| !s.is_empty()) {
            application.on_key(k);
        }
    }

    let mut last_poll = Instant::now();
    let mut dirty = true;

    loop {
        while let Ok(key) = rx.try_recv() {
            application.on_key(&key);
            dirty = true;
        }
        if application.should_quit() {
            break;
        }

        if last_poll.elapsed() >= Duration::from_millis(POLL_INTERVAL_MS) {
            application.poll();
            last_poll = Instant::now();
            dirty = true;
        }

        if dirty {
            if application.wants_model() {
                application.rebuild_model();
            }
            dirty = false;
        }

        application.sync();
        application.tick();
        display.pump();

        std::thread::sleep(Duration::from_millis(FRAME_SLEEP_MS));
    }
    Ok(())
}
