use super::TextureKind;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct SkieImageId(pub usize);

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct SkieImage {
    pub(crate) image_id: SkieImageId,
    pub(crate) texture_kind: TextureKind,
}

impl SkieImage {
    pub const WHITE_IMAGE: Self = Self {
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
