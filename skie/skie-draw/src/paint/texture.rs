use crate::math::{DevicePixels, Size};

use super::atlas::AtlasTextureId;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum TextureId {
    /// Internal things created by skie
    Internal(usize),
    // Any texture created by user
    User(usize),
    // Auto generated by AtlasManager
    AtlasTile(usize),
    // Used by renderer internally to decouple renderer from using the atlas system
    AtlasTexture(AtlasTextureId),
}

impl Default for TextureId {
    fn default() -> Self {
        Self::WHITE_TEXTURE
    }
}

impl TextureId {
    pub const WHITE_TEXTURE: Self = TextureId::Internal(1);
    #[inline(always)]
    pub fn is_white(&self) -> bool {
        self == &Self::WHITE_TEXTURE
    }
}

pub static WHITE_UV: (f32, f32) = (0.0, 0.0);

pub struct Texture2DSpecs {
    pub size: Size<DevicePixels>,
    pub format: TextureFormat,
}

pub struct Texture2D {}

impl std::fmt::Display for TextureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal(id) => write!(f, "Texture::Internal({})", id),
            Self::User(id) => write!(f, "Texture::User({})", id),
            Self::AtlasTile(id) => write!(f, "Texture::AtlasTile({})", id),
            Self::AtlasTexture(id) => write!(f, "Texture::AtlasTexture({})", id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq)]
#[repr(usize)]
pub enum TextureKind {
    Grayscale = 0,
    Color = 1,
}

impl TextureKind {
    pub fn get_texture_format(&self) -> TextureFormat {
        match self {
            Self::Grayscale => TextureFormat::R8Unorm,
            // FIXME: should we use Bgara ?
            Self::Color => TextureFormat::Rgba8Unorm,
        }
    }

    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            TextureKind::Color => 4,
            TextureKind::Grayscale => 1,
        }
    }

    pub fn is_color(&self) -> bool {
        matches!(self, Self::Color)
    }

    pub fn is_gray(&self) -> bool {
        matches!(self, Self::Color)
    }
}

impl std::fmt::Display for TextureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Color => write!(f, "Kind::Color"),
            Self::Grayscale => write!(f, "Kind::Gray"),
        }
    }
}

pub type TextureFormat = wgpu::TextureFormat;
pub type WgpuTexture = wgpu::Texture;
pub type WgpuTextureView = wgpu::TextureView;
