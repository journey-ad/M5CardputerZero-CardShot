// CardShot 设备端界面入口：平台初始化与主循环

mod app;
mod constants;
mod daemon;
mod fonts;
mod i18n;
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
    input::spawn_keys(input::KEYBOARD_PATH.to_string(), tx);

    let mut application = App::new(ui);
    application.init();
    // 环境变量 CARDSHOT_TEST_KEYS 在启动后注入按键序列
    if let Ok(keys) = std::env::var("CARDSHOT_TEST_KEYS") {
        for key in keys.split(',').filter(|part| !part.is_empty()) {
            application.on_key(key);
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
