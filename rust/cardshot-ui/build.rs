#[path = "build/font_cache.rs"]
mod font_cache;
#[path = "build/ui_fonts.rs"]
mod ui_fonts;

fn main() {
    font_cache::generate(
        "ui_fonts",
        &[
            "fonts/fusion8.ttf",
            "fonts/fusion10.ttf",
            "fonts/fusion12.ttf",
            "build/ui_fonts.rs",
        ],
        &["ui_fonts.rs", "ui_fonts.bin"],
        ui_fonts::generate,
    );
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer);
    slint_build::compile_with_config("ui/app.slint", config).unwrap();
}
