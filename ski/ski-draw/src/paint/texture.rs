use crate::math::{DevicePixels, Size};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum TextureId {
    Internal(usize),
    User(usize),
    AtlasTile {},
}

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
            Self::AtlasTile { .. } => {
                write!(f, "Texture::AtlasTile()")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum TextureKind {
    Grayscale = 0,
    Color = 1,
}

impl TextureKind {
    pub fn get_format(&self) -> TextureFormat {
        match self {
            Self::Grayscale => TextureFormat::R8Unorm,
            // FIXME should we use Bgara ?
            Self::Color => TextureFormat::Rgba8UnormSrgb,
        }
    }

    pub fn is_color(&self) -> bool {
        matches!(self, Self::Color)
    }

    pub fn is_gray(&self) -> bool {
        matches!(self, Self::Color)
    }
}

pub type TextureFormat = wgpu::TextureFormat;
pub type WgpuTexture = wgpu::Texture;
