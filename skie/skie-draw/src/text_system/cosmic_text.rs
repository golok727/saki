use std::borrow::Cow;
use std::sync::Arc;

use ahash::HashMap;
use anyhow::{anyhow, Result};
use anyhow::{bail, Context};
use cosmic_text::CacheKey;
use cosmic_text::CacheKeyFlags;
use cosmic_text::Font as CosmicTextFont;
use cosmic_text::FontSystem;
use cosmic_text::SwashCache;
use parking_lot::RwLock;
use smallvec::SmallVec;

use crate::{arc_string::ArcString, Rect};

use super::FontProvider;
use super::GlyphId;
use super::{Font, FontId};

#[derive(Debug)]
pub struct CosmicTextProvider(RwLock<CosmicTextProviderState>);

impl Default for CosmicTextProvider {
    fn default() -> Self {
        Self(RwLock::new(CosmicTextProviderState {
            font_system: FontSystem::new(),
            loaded_fonts: Vec::new(),
            swash_cache: SwashCache::new(),
            font_family_to_id_cache: Default::default(),
        }))
    }
}

#[derive(Debug)]
struct CosmicTextProviderState {
    loaded_fonts: Vec<Arc<CosmicTextFont>>,
    font_family_to_id_cache: HashMap<ArcString, SmallVec<[FontId; 4]>>,
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl FontProvider for CosmicTextProvider {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> anyhow::Result<()> {
        self.0.write().add_fonts(fonts)
    }

    fn font_id(&self, font: &Font) -> Result<FontId> {
        let mut state = self.0.write();

        let variants = if let Some(ids) = state.font_family_to_id_cache.get(&font.family) {
            ids.as_slice()
        } else {
            let ids = state.load(&font.family)?;
            state
                .font_family_to_id_cache
                .insert(font.family.clone(), ids);
            state.font_family_to_id_cache[&font.family].as_ref()
        };

        let props = variants
            .iter()
            .map(|f| {
                let font = state.loaded_fonts[f.0].as_ref();
                let face = state
                    .font_system
                    .db()
                    .face(font.id())
                    .expect("cant get font face");

                (face.style, face.weight)
            })
            .collect::<SmallVec<[_; 4]>>();

        let best_match_idx = best_match::find_best_match(&props, font);
        // FIXME: INDEX OUT OF BOUNDS
        Ok(variants[best_match_idx])
    }

    fn glyph_for_char(&self, font_id: FontId, character: char) -> Option<GlyphId> {
        self.0.write().glyph_id_for_char(font_id, character)
    }

    fn rasterize(&self, specs: &super::GlyphRenderSpecs) -> Result<(Rect<i32>, Vec<u8>)> {
        self.0.write().rasterize_glyph(specs)
    }
}

impl CosmicTextProviderState {
    fn load(&mut self, familiy: &str) -> Result<SmallVec<[FontId; 4]>> {
        let mut font_ids = SmallVec::new();
        let families = self
            .font_system
            .db()
            .faces()
            .filter(|f| f.families.iter().any(|f| f.0 == familiy))
            .map(|f| f.id)
            .collect::<SmallVec<[_; 4]>>();

        if families.is_empty() {
            bail!("No faces found for family: {familiy}")
        }

        for font_id in families {
            let font = self
                .font_system
                .get_font(font_id)
                .ok_or_else(|| anyhow!("Error loading font"))?;

            let font_id = FontId(self.loaded_fonts.len());
            font_ids.push(font_id);
            self.loaded_fonts.push(font);
        }

        Ok(font_ids)
    }

    fn rasterize_glyph(&mut self, specs: &super::GlyphRenderSpecs) -> Result<(Rect<i32>, Vec<u8>)> {
        let font = &self.loaded_fonts[specs.font_id.0];
        let image = self
            .swash_cache
            .get_image(
                &mut self.font_system,
                CacheKey::new(
                    font.id(),
                    specs.glyph_id.0 as u16,
                    specs.font_size * specs.scale_factor,
                    (0.0, 0.0),
                    CacheKeyFlags::empty(),
                )
                .0,
            )
            .clone()
            .with_context(|| r#"No image found"#.to_string())?;

        let bounds = Rect::new(
            image.placement.left,
            image.placement.top,
            image.placement.width as i32,
            image.placement.height as i32,
        );

        Ok((bounds, image.data))
    }

    fn glyph_id_for_char(&self, font_id: FontId, character: char) -> Option<GlyphId> {
        let glyph_idx = self.loaded_fonts[font_id.0]
            .as_swash()
            .charmap()
            .map(character);

        if glyph_idx == 0 {
            None
        } else {
            Some(GlyphId(glyph_idx.into()))
        }
    }

    fn add_fonts(&mut self, fonts: Vec<Cow<'static, [u8]>>) -> anyhow::Result<()> {
        let db = self.font_system.db_mut();
        for font_data in fonts {
            match font_data {
                Cow::Borrowed(data) => db.load_font_data(data.to_vec()),
                Cow::Owned(data) => db.load_font_data(data),
            }
        }
        Ok(())
    }
}

mod best_match {
    use crate::Font;
    use crate::FontStyle;
    use crate::FontWeight;

    use cosmic_text::Style as CosmicStyle;
    use cosmic_text::Weight as CosmicWeight;

    impl FontWeight {
        fn to_cosmic_weight(self) -> CosmicWeight {
            match self {
                FontWeight::THIN => CosmicWeight::THIN,
                FontWeight::LIGHT => CosmicWeight::LIGHT,
                FontWeight::NORMAL => CosmicWeight::NORMAL,
                FontWeight::MEDIUM => CosmicWeight::MEDIUM,
                FontWeight::BOLD => CosmicWeight::BOLD,
                FontWeight::BLACK => CosmicWeight::BLACK,
                other => CosmicWeight(other.0 as u16),
            }
        }
    }

    impl FontStyle {
        fn to_cosmic_style(self) -> CosmicStyle {
            match self {
                FontStyle::Normal => CosmicStyle::Normal,
                FontStyle::Italic => CosmicStyle::Italic,
                FontStyle::Oblique => CosmicStyle::Oblique,
            }
        }
    }

    pub fn find_best_match(properties: &[(CosmicStyle, CosmicWeight)], font: &Font) -> usize {
        let target_style = font.style.to_cosmic_style();
        let target_weight = font.weight.to_cosmic_weight();

        properties
            .iter()
            .enumerate()
            .min_by_key(|(_, &(style, weight))| {
                let style_diff = if style == target_style { 0 } else { 1 };
                let weight_diff = (weight.0 - target_weight.0) as i32;
                style_diff * 100 + weight_diff
            })
            .map(|(index, _)| index)
            .unwrap_or(0)
    }
}
