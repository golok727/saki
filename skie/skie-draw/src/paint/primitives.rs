use crate::{math::Corners, traits::Zero};
use std::fmt::Debug;

use crate::math::{Rect, Vec2};

use super::{path::Path2D, Color, TextureId};

#[derive(Debug, Clone)]
pub enum PrimitiveKind {
    Quad(Quad),
    Path(Path2D),
    Circle(Circle),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillStyle {
    pub color: Color,
}

impl Default for FillStyle {
    fn default() -> Self {
        Self {
            color: Color::TRANSPARENT,
        }
    }
}

impl FillStyle {
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineJoin {
    Miter,
    Bevel,
    Round,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineCap {
    Round,
    Square,
    Butt,
    Joint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrokeStyle {
    pub color: Color,
    pub line_width: u32,
    pub line_join: LineJoin,
    pub line_cap: LineCap,
    pub allow_overlap: bool,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            line_width: 2,
            line_join: LineJoin::Miter,
            line_cap: LineCap::Butt,
            allow_overlap: false,
        }
    }
}

impl StrokeStyle {
    pub fn allow_overlap(mut self, allow: bool) -> Self {
        self.allow_overlap = allow;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn line_width(mut self, line_width: u32) -> Self {
        self.line_width = line_width;
        self
    }

    pub fn line_join(mut self, line_join: LineJoin) -> Self {
        self.line_join = line_join;
        self
    }

    pub fn line_cap(mut self, line_cap: LineCap) -> Self {
        self.line_cap = line_cap;
        self
    }

    pub fn default_join(mut self) -> Self {
        self.line_join = LineJoin::Miter;
        self
    }

    pub fn miter_join(mut self) -> Self {
        self.line_join = LineJoin::Miter;
        self
    }

    pub fn bevel_join(mut self) -> Self {
        self.line_join = LineJoin::Bevel;
        self
    }

    pub fn round_join(mut self) -> Self {
        self.line_join = LineJoin::Round;
        self
    }

    pub fn round_cap(mut self) -> Self {
        self.line_cap = LineCap::Round;
        self
    }

    /// aka with_butt_join lol
    pub fn default_cap(mut self) -> Self {
        self.line_cap = LineCap::Butt;
        self
    }

    pub fn square_cap(mut self) -> Self {
        self.line_cap = LineCap::Square;
        self
    }

    /// sets line cap to join which will join the last point to first point
    pub fn join(mut self) -> Self {
        self.line_cap = LineCap::Joint;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Primitive {
    pub kind: PrimitiveKind,
    pub texture: TextureId,
    pub fill: FillStyle,
    pub stroke: Option<StrokeStyle>,
}

impl Primitive {
    #[inline(always)]
    pub(crate) fn can_render(&self) -> bool {
        let stroke_color = self.stroke.map_or(Color::TRANSPARENT, |s| s.color);
        !self.fill.color.is_transparent() || !stroke_color.is_transparent()
    }

    pub fn textured(mut self, tex: &TextureId) -> Self {
        self.texture = tex.clone();
        if self.fill.color.is_transparent() {
            self.fill.color = Color::WHITE;
        }
        self
    }

    pub fn no_fill(mut self) -> Self {
        self.fill.color = Color::TRANSPARENT;
        self
    }

    pub fn no_stroke(mut self) -> Self {
        self.stroke = None;
        self
    }

    pub fn fill(mut self, style: FillStyle) -> Self {
        self.fill = style;
        self
    }

    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill.color = color;
        self
    }

    pub fn stroke(mut self, style: StrokeStyle) -> Self {
        self.stroke.replace(style);
        self
    }

    pub fn stroke_color(mut self, color: Color) -> Self {
        let stroke = self.stroke.get_or_insert(Default::default());
        stroke.color = color;
        self
    }

    pub fn stroke_width(mut self, width: u32) -> Self {
        let stroke = self.stroke.get_or_insert(Default::default());
        stroke.line_width = width;
        self
    }

    pub fn line_join(mut self, join: LineJoin) -> Self {
        let stroke = self.stroke.get_or_insert(Default::default());
        stroke.line_join = join;
        self
    }

    pub fn line_cap(mut self, cap: LineCap) -> Self {
        let stroke = self.stroke.get_or_insert(Default::default());
        stroke.line_cap = cap;
        self
    }
}

#[derive(Debug, Default, Clone)]
pub struct Circle {
    pub center: Vec2<f32>,
    pub radius: f32,
}

impl Circle {
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn pos(mut self, cx: f32, cy: f32) -> Self {
        self.center.x = cx;
        self.center.y = cy;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Quad {
    pub bounds: Rect<f32>,
    pub corners: Corners<f32>,
}

impl Quad {
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.bounds.size.width = width;
        self.bounds.size.height = height;
        self
    }

    pub fn pos(mut self, x: f32, y: f32) -> Self {
        self.bounds.origin.x = x;
        self.bounds.origin.y = y;
        self
    }

    pub fn rect(mut self, rect: Rect<f32>) -> Self {
        self.bounds = rect;
        self
    }

    pub fn corners(mut self, corners: Corners<f32>) -> Self {
        self.corners = corners;
        self
    }
}

impl Default for Quad {
    fn default() -> Self {
        Self {
            bounds: Rect::zero(),
            corners: Corners::default(),
        }
    }
}

#[inline]
pub fn quad() -> Quad {
    Quad::default()
}

#[inline]
pub fn circle() -> Circle {
    Circle::default()
}

impl From<Quad> for PrimitiveKind {
    #[inline]
    fn from(quad: Quad) -> Self {
        PrimitiveKind::Quad(quad)
    }
}

impl From<Circle> for PrimitiveKind {
    #[inline]
    fn from(circle: Circle) -> Self {
        PrimitiveKind::Circle(circle)
    }
}

impl From<Path2D> for PrimitiveKind {
    #[inline]
    fn from(path: Path2D) -> Self {
        PrimitiveKind::Path(path)
    }
}

impl<T> From<T> for Primitive
where
    T: Into<PrimitiveKind>,
{
    fn from(value: T) -> Self {
        Primitive {
            kind: value.into(),
            texture: TextureId::WHITE_TEXTURE,
            fill: FillStyle::default(),
            stroke: None,
        }
    }
}

pub trait AsPrimitive {
    fn primitive(self) -> Primitive;
}

impl<T> AsPrimitive for T
where
    T: Into<Primitive>,
{
    fn primitive(self) -> Primitive {
        self.into()
    }
}
