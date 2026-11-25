use std::path::PathBuf;

pub fn expand_home(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    if p == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(p)
}

pub fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// 按本地时区用 strftime 格式化当前时间
pub fn local_time_fmt(fmt: &str) -> String {
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        let cfmt = match std::ffi::CString::new(fmt) {
            Ok(c) => c,
            Err(_) => return String::new(),
        };
        let mut buf = vec![0u8; 128];
        let n = libc::strftime(
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
            cfmt.as_ptr(),
            &tm,
        );
        buf.truncate(n);
        String::from_utf8_lossy(&buf).into_owned()
    }
}

pub fn human_size(n: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} B", n)
    } else {
        format!("{:.1} {}", v, U[i])
    }
}
