use std::collections::HashSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

fn collect_source_chars(dir: &Path, out: &mut HashSet<char>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_source_chars(&path, out);
            continue;
        }
        let ext = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("");
        if !matches!(ext, "slint" | "rs" | "po") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for ch in text.chars() {
            if (' '..='~').contains(&ch) || ch as u32 >= 0x2000 {
                out.insert(ch);
            }
        }
    }
}

fn used_chars() -> HashSet<char> {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut out: HashSet<char> = (' '..='~').collect();
    for sub in ["ui", "src", "i18n"] {
        collect_source_chars(&root.join(sub), &mut out);
    }
    out
}

pub fn generate() {
    let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let mut code = String::new();
    let mut pixels = Vec::new();
    // 只光栅化源码中出现的字符
    for size in [12, 10, 8] {
        let source = format!(
            "{}/fonts/fusion{size}.ttf",
            std::env::var("CARGO_MANIFEST_DIR").unwrap()
        );
        let font = fontdue::Font::from_bytes(
            std::fs::read(source).expect("读取 UI 字体失败"),
            fontdue::FontSettings::default(),
        )
        .expect("解析 UI 字体失败");
        let used = used_chars();
        let mut characters: Vec<char> = font
            .chars()
            .keys()
            .copied()
            .filter(|character| used.contains(character))
            .collect();
        // 对象替换字符预留 12px 表情与左右各 1px 字距，表情由 Text 绘制
        characters.retain(|&character| character != '\u{fffc}');
        characters.push('\u{fffc}');
        characters.sort_unstable();
        assert!(characters.len() < u16::MAX as usize);
        writeln!(code, "static MAP_{size}: &[CharacterMapEntry] = &[").unwrap();
        for (index, character) in characters.iter().enumerate() {
            writeln!(
                code,
                "CharacterMapEntry {{ code_point: {character:?}, glyph_index: {index} }},"
            )
            .unwrap();
        }
        code.push_str("];\n");
        // 8px 字体同时提供两倍字号，关于页项目名使用 16px 字形
        let sizes: &[u16] = if size == 8 {
            &[8, 16]
        } else {
            std::slice::from_ref(&size)
        };
        let mut glyph_sets = Vec::new();
        for &pixel_size in sizes {
            writeln!(code, "static GLYPHS_{pixel_size}: &[BitmapGlyph] = &[").unwrap();
            for &character in &characters {
                if character == '\u{fffc}' {
                    code.push_str("BitmapGlyph { x: 0, y: 0, width: 0, height: 0, x_advance: 14, data: glyph_pixels(0, 0) },\n");
                    continue;
                }
                let (metrics, bitmap) = font.rasterize(character, pixel_size as f32);
                encode(&mut code, &mut pixels, metrics, &bitmap);
            }
            code.push_str("];\n");
            glyph_sets.push(format!("BitmapGlyphs {{ pixel_size: {pixel_size}, glyph_data: Slice::from_slice(GLYPHS_{pixel_size}) }}"));
        }
        let metrics = font.horizontal_line_metrics(font.units_per_em()).unwrap();
        writeln!(
            code,
            "static FONT_{size}: BitmapFont = BitmapFont {{
            family_name: Slice::from_slice(b\"Fusion Pixel {size}px Prop zh-Hans\"),
            character_map: Slice::from_slice(MAP_{size}),
            units_per_em: {:?}, ascent: {:?}, descent: {:?},
            glyphs: Slice::from_slice(&[{}]),
            weight: 400, italic: false,
        }};",
            font.units_per_em(),
            metrics.ascent,
            metrics.descent,
            glyph_sets.join(", ")
        )
        .unwrap();
    }
    code.push_str("static FONTS: &[&BitmapFont] = &[&FONT_12, &FONT_10, &FONT_8];\n");
    std::fs::write(output.join("ui_fonts.rs"), code).unwrap();
    std::fs::write(output.join("ui_fonts.bin"), pixels).unwrap();
}

fn encode(code: &mut String, pixels: &mut Vec<u8>, metrics: fontdue::Metrics, bitmap: &[u8]) {
    let offset = pixels.len();
    pixels.extend_from_slice(bitmap);
    writeln!(code, "BitmapGlyph {{ x: {}, y: {}, width: {}, height: {}, x_advance: {}, data: glyph_pixels({offset}, {}) }},",
        metrics.xmin, metrics.ymin, metrics.width, metrics.height, metrics.advance_width.round() as i16, bitmap.len()).unwrap();
}
