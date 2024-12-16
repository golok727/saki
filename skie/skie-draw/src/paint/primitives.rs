use std::cell::Cell;
use std::fmt::Debug;

use crate::math::{Rect, Vec2};
use crate::traits::IsZero;

use super::Color;

#[derive(Debug, Clone)]
pub enum PrimitiveKind {
    Quad(Quad),
    Path(Path2D),
    Circle(Circle),
}

#[derive(Debug, Default, Clone)]
pub struct Circle {
    pub center: Vec2<f32>,
    pub radius: f32,
    pub background_color: Color,
}

impl Circle {
    pub fn with_bgcolor(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

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
    pub background_color: Color,
    pub corners: Corners<f32>,
}

impl Quad {
    pub fn with_bgcolor(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }
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
            background_color: Color::WHITE,
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
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Corners<T: Clone + Default + Debug> {
    pub top_left: T,
    pub top_right: T,
    pub bottom_left: T,
    pub bottom_right: T,
}

impl<T> Corners<T>
where
    T: Clone + Debug + Default,
{
    pub fn with_each(top_left: T, top_right: T, bottom_left: T, bottom_right: T) -> Self {
        Self {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }

    pub fn with_all(v: T) -> Self {
        Self {
            top_left: v.clone(),
            top_right: v.clone(),
            bottom_left: v.clone(),
            bottom_right: v,
        }
    }

    pub fn with_top_left(mut self, v: T) -> Self {
        self.top_left = v;
        self
    }

    pub fn with_top_right(mut self, v: T) -> Self {
        self.top_right = v;
        self
    }

    pub fn with_bottom_left(mut self, v: T) -> Self {
        self.bottom_left = v;
        self
    }

    pub fn with_bottom_right(mut self, v: T) -> Self {
        self.bottom_right = v;
        self
    }
}

impl<T> IsZero for Corners<T>
where
    T: IsZero + Clone + Debug + Default,
{
    fn is_zero(&self) -> bool {
        self.top_left.is_zero()
            && self.top_right.is_zero()
            && self.bottom_left.is_zero()
            && self.bottom_right.is_zero()
    }
}
