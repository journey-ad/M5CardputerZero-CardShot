// 字形结构与当前锁定的 Slint 1.8 软件渲染器一致
use slint::private_unstable_api::re_exports::{
    BitmapFont, BitmapGlyph, BitmapGlyphs, CharacterMapEntry, Slice,
};
use i_slint_core::textlayout::TextShaper;

static PIXELS: [u8; include_bytes!(concat!(env!("OUT_DIR"), "/ui_fonts.bin")).len()] =
    *include_bytes!(concat!(env!("OUT_DIR"), "/ui_fonts.bin"));
include!(concat!(env!("OUT_DIR"), "/ui_fonts.rs"));

const fn glyph_pixels(offset: usize, length: usize) -> Slice<'static, u8> {
    Slice::from_slice(PIXELS.split_at(offset).1.split_at(length).0)
}

pub fn register(window: &dyn slint::platform::WindowAdapter) {
    for font in FONTS {
        window.renderer().register_bitmap_font(font);
    }
}

// 表情位置使用与 Slint 位图字体相同的整数度量
pub(crate) struct LayoutFont {
    font: &'static BitmapFont,
    glyphs: &'static BitmapGlyphs,
}

impl LayoutFont {
    pub(crate) fn new(family: &str, size: f32) -> Self {
        let font = FONTS
            .iter()
            .copied()
            .find(|font| font.family_name.as_slice() == family.as_bytes())
            .unwrap_or(FONTS[0]);
        // 与软件渲染器一致，选择不超过请求字号的最大字阶
        let index = font
            .glyphs
            .partition_point(|glyphs| f32::from(glyphs.pixel_size) <= size)
            .saturating_sub(1);
        Self {
            font,
            glyphs: &font.glyphs[index],
        }
    }

    // 累计文本的像素宽度
    pub(crate) fn advance(&self, text: &str) -> i16 {
        text.chars()
            .map(|character| {
                self.glyph_for_char(character)
                    .map(|glyph| glyph.advance)
                    .unwrap_or(self.glyphs.pixel_size)
            })
            .sum()
    }
}

// 界面默认位图字体与页签字号
pub(crate) const UI_FONT_FAMILY: &str = "Fusion Pixel 12px Prop zh-Hans";
pub(crate) const UI_FONT_SIZE: f32 = 12.0;

impl i_slint_core::textlayout::FontMetrics<i16> for LayoutFont {
    fn ascent(&self) -> i16 {
        (f32::from(self.glyphs.pixel_size) * self.font.ascent / self.font.units_per_em) as i16
    }
    fn descent(&self) -> i16 {
        (f32::from(self.glyphs.pixel_size) * self.font.descent / self.font.units_per_em) as i16
    }
    fn height(&self) -> i16 {
        (f32::from(self.glyphs.pixel_size) * (self.font.ascent - self.font.descent)
            / self.font.units_per_em) as i16
    }
}

impl i_slint_core::textlayout::TextShaper for LayoutFont {
    type LengthPrimitive = i16;
    type Length = i16;

    fn shape_text<Storage: Extend<i_slint_core::textlayout::Glyph<i16>>>(
        &self,
        text: &str,
        glyphs: &mut Storage,
    ) {
        glyphs.extend(text.char_indices().map(|(offset, character)| {
            let mut glyph =
                self.glyph_for_char(character)
                    .unwrap_or_else(|| i_slint_core::textlayout::Glyph {
                        advance: self.glyphs.pixel_size,
                        ..Default::default()
                    });
            glyph.text_byte_offset = offset;
            glyph
        }));
    }

    fn glyph_for_char(&self, character: char) -> Option<i_slint_core::textlayout::Glyph<i16>> {
        let index = self
            .font
            .character_map
            .binary_search_by_key(&character, |entry| entry.code_point)
            .ok()?;
        let index = self.font.character_map[index].glyph_index as usize;
        Some(i_slint_core::textlayout::Glyph {
            glyph_id: std::num::NonZeroU16::new(index as u16 + 1),
            advance: self.glyphs.glyph_data[index].x_advance,
            ..Default::default()
        })
    }

    fn max_lines(&self, max_height: i16) -> usize {
        use i_slint_core::textlayout::FontMetrics;
        (max_height / self.height()) as usize
    }
}
