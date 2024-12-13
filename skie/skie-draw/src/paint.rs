pub mod atlas;
pub mod color;
pub mod draw_list;
pub mod primitives;
pub mod texture;

pub use color::*;
pub use draw_list::*;
pub use primitives::*;
pub use texture::*;

use crate::math::Vec2;

pub const DEFAULT_UV_COORD: Vec2<f32> = Vec2 { x: 0.0, y: 0.0 };
