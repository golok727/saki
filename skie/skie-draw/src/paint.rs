pub mod atlas;
pub mod color;
pub mod draw_list;
pub mod image;
pub mod mesh;
pub mod path;
pub mod polyline;
pub mod primitives;
pub mod text;
pub mod texture;

use atlas::{AtlasKeyImpl, TextureAtlas};

use crate::{math::Vec2, text::GlyphImage};

pub use color::*;
pub use draw_list::*;
pub use image::*;
pub use mesh::*;
pub use polyline::*;
pub use primitives::*;
pub use text::*;
pub use texture::*;

pub const DEFAULT_UV_COORD: Vec2<f32> = Vec2 { x: 0.0, y: 0.0 };

pub type SkieAtlas = TextureAtlas<AtlasKey>;
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum AtlasKey {
    Image(SkieImage),
    Glyf(GlyphImage),
}

impl From<GlyphImage> for AtlasKey {
    fn from(atlas_glyf: GlyphImage) -> Self {
        Self::Glyf(atlas_glyf)
    }
}

impl From<SkieImage> for AtlasKey {
    fn from(image: SkieImage) -> Self {
        Self::Image(image)
    }
}

impl AtlasKeyImpl for AtlasKey {
    const WHITE_TEXTURE_KEY: Self = Self::Image(SkieImage::WHITE_IMAGE);

    fn kind(&self) -> TextureKind {
        match self {
            AtlasKey::Glyf(glyph) => {
                if glyph.is_emoji {
                    TextureKind::Color
                } else {
                    TextureKind::Mask
                }
            }
            AtlasKey::Image(image) => image.texture_kind,
        }
    }
}
