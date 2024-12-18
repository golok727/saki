use crate::math::Corners;
use std::cell::Cell;
use std::fmt::Debug;

use crate::math::{Rect, Vec2};

use super::{Color, TextureId};

#[derive(Debug, Clone)]
pub enum PrimitiveKind {
    Quad(Quad),
    Path(Path2D),
    Circle(Circle),
}

#[derive(Debug, Default, Clone)]
pub struct FillStyle {
    pub color: Color,
}

impl FillStyle {
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

#[derive(Debug, Clone)]
pub enum LineJoin {
    Mitier,
    Square,
    Round,
}

#[derive(Debug, Clone)]
pub enum LineCap {
    Mitier,
    Square,
    Butt,
}

#[derive(Debug, Clone)]
pub struct StrokeStyle {
    pub color: Color,
    pub line_width: u16,
    pub line_join: LineJoin,
    pub line_cap: LineCap,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            color: Default::default(),
            line_width: 2,
            line_join: LineJoin::Square,
            line_cap: LineCap::Butt,
        }
    }
}

impl StrokeStyle {
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_line_width(mut self, line_width: u16) -> Self {
        self.line_width = line_width;
        self
    }

    pub fn with_line_join(mut self, line_join: LineJoin) -> Self {
        self.line_join = line_join;
        self
    }

    pub fn with_line_cap(mut self, line_cap: LineCap) -> Self {
        self.line_cap = line_cap;
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
    pub fn textured(mut self, tex: TextureId) -> Self {
        self.texture = tex;
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

    pub fn with_fill_color(mut self, color: Color) -> Self {
        self.fill.color = color;
        self
    }

    pub fn stroke(mut self, style: StrokeStyle) -> Self {
        self.stroke.replace(style);
        self
    }

    pub fn with_stroke_color(mut self, color: Color) -> Self {
        let stroke = self.stroke.get_or_insert(Default::default());
        stroke.color = color;
        self
    }

    pub fn with_line_width(mut self, width: u16) -> Self {
        let stroke = self.stroke.get_or_insert(Default::default());
        stroke.line_width = width;
        self
    }

    pub fn with_line_join(mut self, join: LineJoin) -> Self {
        let stroke = self.stroke.get_or_insert(Default::default());
        stroke.line_join = join;
        self
    }

    pub fn with_line_cap(mut self, cap: LineCap) -> Self {
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
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_pos(mut self, cx: f32, cy: f32) -> Self {
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
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.bounds.width = width;
        self.bounds.height = height;
        self
    }

    pub fn with_pos(mut self, x: f32, y: f32) -> Self {
        self.bounds.x = x;
        self.bounds.y = y;
        self
    }

    pub fn with_corners(mut self, corners: Corners<f32>) -> Self {
        self.corners = corners;
        self
    }

    pub(crate) fn into_primitive_kind(self) -> PrimitiveKind {
        // TODO: use path if its a rounded rectangle
        PrimitiveKind::Quad(self)
    }
}

impl Default for Quad {
    fn default() -> Self {
        Self {
            bounds: Rect {
                x: 0.,
                y: 0.,
                width: 10.,
                height: 10.,
            },
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

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct PathId(pub(crate) usize);

pub static DEFAULT_PATH_ID: PathId = PathId(0);

#[derive(Debug, Clone)]
pub enum PathOp {
    MoveTo(Vec2<f32>),
    LineTo(Vec2<f32>),
    QuadratcBezierTo {
        control: Vec2<f32>,
        to: Vec2<f32>,
    },
    ArcTo {
        center: Vec2<f32>,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        clockwise: bool,
    },
    ClosePath,
}

#[derive(Debug, Default, Clone)]
pub struct Path2D {
    pub id: PathId,
    pub(crate) ops: Vec<PathOp>,
    pub(crate) dirty: Cell<bool>,
    // pub(crate) flags
}

impl Path2D {
    #[allow(unused)]
    pub(crate) fn with_flags(&mut self) {
        todo!()
    }

    pub fn move_to(&mut self, to: Vec2<f32>) {
        self.dirty.set(true);
        self.ops.push(PathOp::MoveTo(to))
    }

    pub fn line_to(&mut self, to: Vec2<f32>) {
        self.dirty.set(true);
        self.ops.push(PathOp::MoveTo(to))
    }

    pub fn quadratic_bezier_to(&mut self, control: Vec2<f32>, to: Vec2<f32>) {
        self.dirty.set(true);
        self.ops.push(PathOp::QuadratcBezierTo { control, to })
    }

    pub fn close_path(&mut self) {
        self.dirty.set(true);
        self.ops.push(PathOp::ClosePath)
    }

    pub fn arc(
        &mut self,
        center: Vec2<f32>,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        clockwise: bool,
    ) {
        self.dirty.set(true);
        self.ops.push(PathOp::ArcTo {
            center,
            radius,
            start_angle,
            end_angle,
            clockwise,
        })
    }

    pub fn clear(&mut self) {
        self.dirty.set(true);
        self.ops.clear()
    }
}

impl From<Quad> for PrimitiveKind {
    #[inline]
    fn from(quad: Quad) -> Self {
        quad.into_primitive_kind()
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

// macro_rules! impl_into_primitive {
//     ($t: ty, $kind: tt) => {
//         impl From<$t> for PrimitiveKind {
//             fn from(val: $t) -> Self {
//                 PrimitiveKind::$kind(val)
//             }
//         }
//     };
// }
//
