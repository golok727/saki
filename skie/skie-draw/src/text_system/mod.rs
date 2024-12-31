use anyhow::Result;
use font_due::FontDueProvider;
use std::{
    borrow::Cow,
    fmt::Debug,
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::{arc_string::ArcString, Rect};
mod font_due;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtlasGlyph {
    pub(crate) font_id: FontId,
    pub(crate) glyph_id: GlyphId,
    /// font-size in pixels
    pub(crate) font_size: f32,
    pub(crate) scale_factor: f32,
}

impl Eq for AtlasGlyph {}

impl Hash for AtlasGlyph {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.font_id.0.hash(state);
        self.glyph_id.0.hash(state);
        self.font_size.to_bits().hash(state);
        self.scale_factor.to_bits().hash(state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphId(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontId(pub(crate) usize);

pub(crate) trait FontProvider: Send + Sync + Debug {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()>;

    fn font_id(&self, font: &Font) -> Option<FontId>;

    fn list_fonts_names(&self) -> Vec<String>;

    fn font_metrics(&self, font_id: FontId) -> FontMetrics;

    fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId>;

    fn rasterize_char(&self, _character: char, _font: &Font) -> Result<(Rect<i32>, Vec<u8>)>;
}

#[derive(Debug)]
pub struct TextSystem {
    pub(crate) provider: Arc<dyn FontProvider>,
}

impl TextSystem {
    pub fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        self.provider.add_fonts(fonts)
    }
}

impl Default for TextSystem {
    fn default() -> Self {
        Self {
            provider: Arc::new(get_font_provider()),
        }
    }
}

#[inline(always)]
fn get_font_provider() -> impl FontProvider {
    FontDueProvider::default()
}

#[derive(Clone, Debug)]
pub struct FontMetrics {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Font {
    pub family: ArcString,
    pub weight: FontWeight,
    pub style: FontStyle,
}

impl Font {
    pub fn bold(mut self) -> Self {
        self.weight = FontWeight::BOLD;
        self
    }

    pub fn italic(mut self) -> Self {
        self.style = FontStyle::Italic;
        self
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct FontWeight(pub f32);

impl Default for FontWeight {
    #[inline]
    fn default() -> FontWeight {
        FontWeight::NORMAL
    }
}

impl Hash for FontWeight {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(u32::from_be_bytes(self.0.to_be_bytes()));
    }
}

impl Eq for FontWeight {}
impl FontWeight {
    pub const THIN: FontWeight = FontWeight(100.0);
    pub const EXTRA_LIGHT: FontWeight = FontWeight(200.0);
    pub const LIGHT: FontWeight = FontWeight(300.0);
    pub const NORMAL: FontWeight = FontWeight(400.0);
    pub const MEDIUM: FontWeight = FontWeight(500.0);
    pub const SEMIBOLD: FontWeight = FontWeight(600.0);
    pub const BOLD: FontWeight = FontWeight(700.0);
    pub const EXTRA_BOLD: FontWeight = FontWeight(800.0);
    pub const BLACK: FontWeight = FontWeight(900.0);
}
