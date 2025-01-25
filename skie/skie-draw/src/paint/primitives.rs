use crate::{math::Corners, Zero};
use std::fmt::Debug;

use crate::math::{Rect, Vec2};

use super::{path::Path2D, Color, FillStyle, LineCap, LineJoin, StrokeStyle, TextureId};

#[derive(Debug, Clone)]
pub enum PrimitiveKind {
    Quad(Quad),
    Path(Path2D),
    Circle(Circle),
}

#[derive(Debug, Clone)]
pub struct Primitive {
    pub kind: PrimitiveKind,
    pub texture: TextureId,
    pub fill: FillStyle,
    pub stroke: Option<StrokeStyle>,
}

impl Primitive {
    pub fn new(prim: impl Into<Self>) -> Self {
        prim.into()
    }

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
