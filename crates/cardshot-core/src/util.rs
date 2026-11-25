use std::path::PathBuf;

pub fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    if path == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(path)
}

pub fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration_secs| duration_secs.as_secs_f64())
        .unwrap_or(0.0)
}

/// 按本地时区用 strftime 格式化当前时间
pub fn local_time_fmt(fmt: &str) -> String {
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        let cfmt = match std::ffi::CString::new(fmt) {
            Ok(cstring) => cstring,
            Err(_) => return String::new(),
        };
        let mut buf = vec![0u8; 128];
        let written = libc::strftime(
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
            cfmt.as_ptr(),
            &tm,
        );
        buf.truncate(written);
        String::from_utf8_lossy(&buf).into_owned()
    }
}

pub fn human_size(written: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = written as f64;
    let mut index = 0;
    while value >= 1024.0 && index < U.len() - 1 {
        value /= 1024.0;
        index += 1;
    }
    if index == 0 {
        format!("{} B", written)
    } else {
        format!("{:.1} {}", value, U[index])
    }
}
