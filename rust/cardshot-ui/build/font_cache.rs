use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

pub fn generate(name: &str, inputs: &[&str], outputs: &[&str], generate: impl FnOnce()) {
    let mut hash = DefaultHasher::new();
    include_bytes!("font_cache.rs").hash(&mut hash);
    for path in inputs.iter().copied().chain(["Cargo.lock"]) {
        println!("cargo:rerun-if-changed={path}");
        std::fs::read(path)
            .expect("读取字体构建输入失败")
            .hash(&mut hash);
    }
    let fingerprint = format!("{:016x}", hash.finish());
    let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let stamp = output.join(format!("{name}.fingerprint"));
    // 字体、生成代码和依赖版本一致时复用构建目录中的字形
    if std::fs::read_to_string(&stamp).ok().as_deref() == Some(fingerprint.as_str())
        && outputs.iter().all(|path| output.join(path).is_file())
    {
        return;
    }
    generate();
    std::fs::write(stamp, fingerprint).expect("保存字体构建缓存失败");
}
