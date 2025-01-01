pub mod atlas;
pub mod color;
pub mod draw_list;
pub mod image;
pub mod mesh;
pub mod path;
pub mod primitives;
pub mod texture;

pub use color::*;
pub use draw_list::*;
pub use image::*;
pub use mesh::*;
pub use primitives::*;
pub use texture::*;

use crate::{math::Vec2, text_system::AtlasGlyph};
use atlas::{AtlasKeyImpl, AtlasManager};

pub const DEFAULT_UV_COORD: Vec2<f32> = Vec2 { x: 0.0, y: 0.0 };

pub type SkieAtlas = AtlasManager<AtlasKey>;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum AtlasKey {
    Image(SkieImage),
    Glyf(AtlasGlyph),
}

impl From<AtlasGlyph> for AtlasKey {
    fn from(atlas_glyf: AtlasGlyph) -> Self {
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
            AtlasKey::Glyf(_) => TextureKind::Grayscale,
            AtlasKey::Image(image) => image.texture_kind,
        }
    }
}
