use std::borrow::Cow;

use anyhow::{anyhow, Result};
use parking_lot::RwLock;

use crate::Rect;

use super::{Font, FontId, FontProvider, GlyphId};

#[derive(Debug, Default)]
pub struct FontDueProvider(RwLock<FontDueProviderState>);

#[derive(Debug, Default)]
struct FontDueProviderState {
    loaded_fonts: Vec<fontdue::Font>,
}

impl FontProvider for FontDueProvider {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        self.0.write().add_fonts(fonts)
    }

    fn list_fonts_names(&self) -> Vec<String> {
        vec![]
    }

    fn font_id(&self, font: &Font) -> Option<FontId> {
        todo!()
    }

    fn font_metrics(&self, _font_id: FontId) -> super::FontMetrics {
        todo!()
    }

    fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId> {
        Some(GlyphId(0))
    }

    fn rasterize_char(&self, _character: char, _font: &Font) -> Result<(Rect<i32>, Vec<u8>)> {
        todo!()
    }
}

impl FontDueProviderState {
    fn add_fonts(&mut self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        let settings = fontdue::FontSettings {
            scale: 30.0,
            ..Default::default()
        };

        for font in fonts {
            let font = match font {
                Cow::Borrowed(bytes) => fontdue::Font::from_bytes(bytes, settings),
                Cow::Owned(bytes) => fontdue::Font::from_bytes(bytes, settings),
            }
            .map_err(|err| anyhow!(err))?; //FIXME: should we continue ?
            if let Some(name) = font.name() {}
        }

        Ok(())
    }
}
