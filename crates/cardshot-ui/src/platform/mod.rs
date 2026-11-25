// 设备平台：软件渲染到 framebuffer

pub mod input;

use std::fs::OpenOptions;
use std::rc::Rc;

use memmap2::MmapMut;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType, Rgb565Pixel};
use slint::platform::{Platform, WindowAdapter};
use slint::PhysicalSize;

use crate::constants::{FB_BYTES, HEIGHT, WIDTH};
use crate::fonts;

pub struct AppPlatform {
    window: Rc<MinimalSoftwareWindow>,
}

impl Platform for AppPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn click_interval(&self) -> std::time::Duration {
        std::time::Duration::ZERO
    }
}

pub struct Display {
    window: Rc<MinimalSoftwareWindow>,
    buffer: Vec<Rgb565Pixel>,
    fbmap: MmapMut,
    file: std::fs::File,
}

impl Display {
    pub fn new() -> std::io::Result<Display> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
        window.set_size(PhysicalSize::new(WIDTH, HEIGHT));
        fonts::register(window.as_ref());
        slint::platform::set_platform(Box::new(AppPlatform {
            window: window.clone(),
        }))
        .expect("set_platform");

        let file = OpenOptions::new().read(true).write(true).open("/dev/fb0")?;
        let fbmap = unsafe { memmap2::MmapOptions::new().len(FB_BYTES).map_mut(&file)? };
        let buffer = vec![Rgb565Pixel(0); FB_BYTES / 2];

        Ok(Display {
            window,
            buffer,
            fbmap,
            file,
        })
    }

    /// 渲染一帧；内容有变化时写回 framebuffer
    pub fn pump(&mut self) -> bool {
        let Display {
            window,
            buffer,
            fbmap,
            file,
        } = self;
        slint::platform::update_timers_and_animations();
        let redrawn = window.draw_if_needed(|renderer| {
            renderer.render(buffer, WIDTH as usize);
        });
        if redrawn {
            let bytes = unsafe { std::slice::from_raw_parts(buffer.as_ptr() as *const u8, FB_BYTES) };
            fbmap[..].copy_from_slice(bytes);
            let _ = file.sync_data();
        }
        redrawn
    }
}
