use anyhow::Result;
use font_due::FontDueProvider;
use std::{
    borrow::Cow,
    fmt::Debug,
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::Rect;
mod font_due;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct FontId(pub usize);

pub(crate) trait FontProvider: Send + Sync + Debug {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()>;

    fn font_id(&self, font: &Font) -> Option<FontId>;

    fn list_fonts_names(&self) -> Vec<String>;

    fn font_metrics(&self, font_id: FontId) -> FontMetrics;

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

fn get_font_provider() -> impl FontProvider {
    FontDueProvider::default()
}

#[derive(Clone, Debug)]
pub struct FontMetrics {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Font {
    family: String,
    weight: FontWeight,
    style: FontStyle,
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
