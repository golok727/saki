pub mod arc_string;
pub mod gpu;
pub mod math;
pub mod paint;
pub mod renderer;
pub mod scene;
pub mod text_system;
pub mod traits;

pub use gpu::{error::GpuContextCreateError, GpuContext};
pub use math::{mat3, vec2, Corners, Mat3, Rect, Size, Vec2};
pub use paint::color::{Color, Rgba};
pub use paint::DrawList;
pub use paint::{
    atlas::{AtlasKeyImpl, AtlasManager, AtlasTextureInfo, AtlasTextureInfoMap},
    circle,
    path::Path2D,
    quad, text, AtlasKey, Circle, FillStyle, LineCap, LineJoin, Primitive, Quad, SkieAtlas,
    StrokeStyle, Text, TextAlign, TextBaseline,
};
pub use paint::{TextureAddressMode, TextureFilterMode, TextureFormat, TextureId, TextureOptions};

pub use renderer::{WgpuRenderer, WgpuRendererSpecs};

pub use scene::Scene;
pub use text_system::{AtlasGlyph, Font, FontId, FontStyle, FontWeight, GlyphId, TextSystem};

pub use traits::{Half, IsZero, Zero};
