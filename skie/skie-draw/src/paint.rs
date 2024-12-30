pub mod atlas;
pub mod color;
pub mod draw_list;
pub mod path;
pub mod primitives;
pub mod texture;

pub use color::*;
pub use draw_list::*;
pub use primitives::*;
pub use texture::*;

use crate::{math::Vec2, text_system::FontId};
use atlas::{AtlasKeyImpl, AtlasManager};

pub const DEFAULT_UV_COORD: Vec2<f32> = Vec2 { x: 0.0, y: 0.0 };

pub type SkieAtlas = AtlasManager<AtlasKey>;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum AtlasKey {
    Image(SkieImage),
    Glyf(AtlasGlyf),
}

impl From<SkieImage> for AtlasKey {
    fn from(image: SkieImage) -> Self {
        Self::Image(image)
    }
}

impl From<AtlasGlyf> for AtlasKey {
    fn from(atlas_glyf: AtlasGlyf) -> Self {
        Self::Glyf(atlas_glyf)
    }
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct SkieImageId(pub usize);

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct SkieImage {
    pub(crate) image_id: SkieImageId,
    pub(crate) texture_kind: TextureKind,
}

impl SkieImage {
    const WHITE_IMAGE: Self = Self {
        image_id: SkieImageId(0),
        texture_kind: TextureKind::Color,
    };
}

impl SkieImage {
    pub fn new(id: usize) -> Self {
        debug_assert_ne!(id, 0, "SkieImageId(0) is reserved for white texture.");
        Self {
            image_id: SkieImageId(id),
            texture_kind: TextureKind::Color,
        }
    }

    pub fn id(&self) -> &SkieImageId {
        &self.image_id
    }

    pub fn texture_kind(&self) -> &TextureKind {
        &self.texture_kind
    }

    pub fn color(mut self) -> Self {
        self.texture_kind = TextureKind::Color;
        self
    }

    pub fn greyscale(mut self) -> Self {
        self.texture_kind = TextureKind::Grayscale;
        self
    }
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct AtlasGlyf {
    pub font_id: FontId,
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
