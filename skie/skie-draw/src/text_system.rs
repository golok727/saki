use anyhow::Result;
use cosmic_text::CosmicTextProvider;
use std::{
    borrow::Cow,
    fmt::Debug,
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::{arc_string::ArcString, Rect};

mod cosmic_text;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphRenderSpecs {
    pub(crate) font_id: FontId,
    pub(crate) glyph_id: GlyphId,
    /// font-size in pixels
    pub(crate) font_size: f32,
    pub(crate) scale_factor: f32,
}

impl Eq for GlyphRenderSpecs {}

impl Hash for GlyphRenderSpecs {
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

pub trait FontProvider: Send + Sync + Debug {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()>;

    fn font_id(&self, font: &Font) -> Result<FontId>;

    fn glyph_id_for_char(&self, font_id: FontId, character: char) -> Option<GlyphId>;

    fn rasterize(&self, specs: &GlyphRenderSpecs) -> Result<(Rect<i32>, Vec<u8>)>;
}

#[derive(Debug)]
pub struct TextSystem {
    pub(crate) provider: Arc<dyn FontProvider>,
}

impl TextSystem {
    /// Creates a text system with the given provider
    /// use TextSystem::default() to use the default provider
    pub fn create(provider: impl FontProvider + 'static) -> Self {
        Self {
            provider: Arc::new(provider),
        }
    }

    pub fn font_id(&self, font: &Font) -> Result<FontId> {
        self.provider.font_id(font)
    }

    pub fn rasterize(&self, specs: &GlyphRenderSpecs) -> Result<(Rect<i32>, Vec<u8>)> {
        self.provider.rasterize(specs)
    }

    pub fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        self.provider.add_fonts(fonts)
    }
}

impl Default for TextSystem {
    fn default() -> Self {
        Self {
            provider: Arc::new(CosmicTextProvider::default()),
        }
    }
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
    pub fn new(family: impl Into<ArcString>) -> Self {
        Self {
            family: family.into(),
            weight: FontWeight::default(),
            style: FontStyle::default(),
        }
    }

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
    Oblique,
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
