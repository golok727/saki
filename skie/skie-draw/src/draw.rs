pub mod arc_string;
pub mod canvas;
pub mod gpu;
pub mod paint;
pub mod renderer;
pub mod scene;
pub mod text;

pub use skie_math as math;

pub use canvas::Canvas;
pub use gpu::{
    Extent3d, GpuContext, GpuContextCreateError, GpuSurface, GpuSurfaceCreateError,
    GpuSurfaceSpecification, GpuTextureDescriptor, GpuTextureDimension, GpuTextureFormat,
    GpuTextureUsages,
};

pub use math::{mat3, vec2, Corners, Mat3, Rect, Size, Vec2};
pub use paint::color::{Color, Rgba};
pub use paint::DrawList;
pub use paint::{
    circle, quad, AtlasKey, AtlasKeySource, AtlasTextureInfo, AtlasTextureInfoMap, Brush, Circle,
    FillStyle, LineCap, LineJoin, Path2D, Primitive, Quad, SkieAtlas, StrokeStyle, Text, TextAlign,
    TextBaseline, TextureAtlas,
};

pub use paint::{
    GpuTexture, GpuTextureView, GpuTextureViewDescriptor, Mesh, TextureAddressMode,
    TextureFilterMode, TextureFormat, TextureId, TextureKind, TextureOptions,
};

pub use renderer::{WgpuRenderer, WgpuRendererSpecs};

pub use scene::Scene;
pub use text::{Font, FontId, FontStyle, FontWeight, GlyphId, GlyphImage, TextSystem};

pub use skie_math::traits::*;

#[cfg(feature = "application")]
pub mod app;
