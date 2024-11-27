pub mod draw_list;
pub mod primitives;

pub use draw_list::*;
pub use primitives::*;

use crate::math::Vec2;

pub const DEFAULT_UV_COORD: Vec2<f32> = Vec2 { x: 0.0, y: 0.0 };

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum TextureId {
    Internal(usize),
    User(usize),
}

impl std::fmt::Display for TextureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal(id) => write!(f, "Texture::Internal({})", id),
            Self::User(id) => write!(f, "Texture::User({})", id),
        }
    }
}
