#[path = "build/font_cache.rs"]
mod font_cache;
#[path = "build/ui_fonts.rs"]
mod ui_fonts;

fn main() {
    for dir in ["ui", "src", "i18n", "fonts"] {
        println!("cargo:rerun-if-changed={dir}");
    }
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let f8 = format!("{manifest_dir}/fonts/fusion8.ttf");
    let f10 = format!("{manifest_dir}/fonts/fusion10.ttf");
    let f12 = format!("{manifest_dir}/fonts/fusion12.ttf");
    let gen_src = format!("{manifest_dir}/build/ui_fonts.rs");
    font_cache::generate(
        "ui_fonts",
        &[f8.as_str(), f10.as_str(), f12.as_str(), gen_src.as_str()],
        &["ui_fonts.rs", "ui_fonts.bin"],
        ui_fonts::generate,
    );
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer);
    slint_build::compile_with_config(format!("{manifest_dir}/ui/app.slint"), config).unwrap();
}
