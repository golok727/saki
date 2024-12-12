use core::f64;
use std::fmt::Debug;

use crate::math::{Rect, Vec2};
use crate::traits::IsZero;

#[derive(Debug, Clone)]
pub enum PrimitiveKind {
    Quad(Quad),
    Path(PathData),
}

#[derive(Debug, Clone)]
pub struct Quad {
    pub bounds: Rect<f32>,
    pub background_color: wgpu::Color,
    pub corners: Corners<f32>,
}

impl Quad {
    pub fn with_bgcolor(mut self, r: f64, g: f64, b: f64, a: f64) -> Self {
        self.background_color = wgpu::Color { r, g, b, a };
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
}

#[derive(Debug, Clone)]
pub struct PathData {
    pub points: Vec<Vec2<f64>>,
    // pub flags: (),
}

impl<T> From<&mut Path<T>> for PathData
where
    T: Debug + Default + Clone + Into<f64>,
{
    fn from(path: &mut Path<T>) -> Self {
        let points = path
            .points
            .iter()
            .map(|point| Vec2 {
                x: point.x.clone().into(),
                y: point.y.clone().into(),
            })
            .collect();

        Self { points }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Path<T: Debug + Default + Clone> {
    pub(crate) points: Vec<Vec2<T>>,
    cursor: Vec2<T>,
    start: Option<Vec2<T>>,
    // todo add flags for path
}

impl<T: Debug + Default + Clone> Path<T> {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(unused)]
    pub(crate) fn with_flags(&mut self) {
        todo!()
    }

    /// Moves the cursor to the specified position without creating a line.
    pub fn move_to(&mut self, new_pos: Vec2<T>) {
        self.cursor = new_pos.clone();
        self.start = Some(new_pos); // Set the start of the new subpath
    }

    /// Draws a straight line from the cursor to the specified position.
    pub fn line_to(&mut self, to: Vec2<T>) {
        self.points.push(self.cursor.clone());
        self.points.push(to.clone());
        self.cursor = to;
    }

    /// Draws a quadratic Bézier curve from the cursor to `to` using `control` as the control point.
    pub fn quadratic_bezier_to(&mut self, _control: Vec2<T>, _to: Vec2<T>) {
        todo!()
    }

    /// Clears all points in the path.
    pub fn clear(&mut self) {
        self.points.clear();
        self.start = None;
    }

    #[allow(unused)]
    pub(crate) fn take(&mut self) -> Vec<Vec2<T>> {
        let val = std::mem::take(&mut self.points);
        self.start = None;
        val
    }

    /// Closes the current subpath by drawing a line back to the starting point.
    pub fn close_path(&mut self) {
        if let Some(start) = &self.start {
            self.line_to(start.clone());
        }
    }
}

impl<T> Path<T>
where
    T: Debug
        + Default
        + Clone
        + std::ops::Mul<Output = T>
        + std::ops::Add<Output = T>
        + From<f64>
        + Into<f64>,
{
    /// Draws an arc.
    pub fn arc(
        &mut self,
        center: Vec2<T>,
        radius: T,
        start_angle: f64,
        end_angle: f64,
        clockwise: bool,
    ) {
        // TODO: make this configurable ?
        const NUM_SEGMENTS: u8 = 32;

        let step: f64 = if clockwise {
            -(end_angle - start_angle) / NUM_SEGMENTS as f64
        } else {
            (end_angle - start_angle) / NUM_SEGMENTS as f64
        };

        for i in 0..=NUM_SEGMENTS {
            let theta = start_angle + i as f64 * step;
            let x = center.x.clone().into() + radius.clone().into() * theta.cos();
            let y = center.y.clone().into() + radius.clone().into() * theta.sin();
            self.points.push(Vec2 {
                x: T::from(x),
                y: T::from(y),
            });
        }

        // Update the cursor to the final point on the arc.
        if let Some(last) = self.points.last() {
            self.cursor = last.clone();
        }
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
            background_color: wgpu::Color::WHITE,
            corners: Corners::default(),
        }
    }
}

pub fn quad() -> Quad {
    Quad::default()
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

impl<T> From<&mut Path<T>> for PrimitiveKind
where
    T: Debug + Default + Clone + Into<f64>,
{
    fn from(value: &mut Path<T>) -> Self {
        PrimitiveKind::Path(value.into())
    }
}

impl<T> From<Path<T>> for PrimitiveKind
where
    T: Debug + Default + Clone + Into<f64>,
{
    fn from(mut value: Path<T>) -> Self {
        PrimitiveKind::Path(PathData::from(&mut value))
    }
}

impl From<Quad> for PrimitiveKind {
    fn from(quad: Quad) -> Self {
        // TODO: add a path insted if it has some corner radius
        PrimitiveKind::Quad(quad)
    }
}

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
    pub fn with_all(v: T) -> Self {
        Self {
            top_left: v.clone(),
            top_right: v.clone(),
            bottom_left: v.clone(),
            bottom_right: v,
        }
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
