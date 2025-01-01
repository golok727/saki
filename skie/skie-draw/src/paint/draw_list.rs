use core::f32;
use std::fmt::Debug;
use std::ops::Range;

use super::path::Path2D;
use super::{Color, Mesh, Polyline, StrokeStyle, Vertex};

use crate::math::{Corners, Rect, Vec2};
use crate::paint::WHITE_UV;

pub type DrawListMiddleware<'a> = Box<dyn Fn(Vertex) -> Vertex + 'a>;

#[derive(Default, Debug)]
pub struct DrawList {
    pub(crate) mesh: Mesh,
    pub(crate) path: Path2D,
}

impl DrawList {
    pub fn clear(&mut self) {
        self.mesh.clear();
        self.path.clear();
    }

    #[inline]
    pub fn capture_range<F>(&mut self, f: F) -> std::ops::Range<usize>
    where
        F: FnOnce(&mut Self),
    {
        let start = self.mesh.vertices.len();
        f(self);
        let end = self.mesh.vertices.len();
        start..end
    }

    #[inline]
    pub fn map_range(&mut self, range: Range<usize>, f: impl Fn(&mut Vertex)) {
        for vertex in &mut self.mesh.vertices[range] {
            f(vertex);
        }
    }

    #[inline]
    pub fn begin_path(&mut self) {
        self.path.clear();
    }

    #[inline]
    pub fn close_path(&mut self) {
        self.path.close();
    }

    #[inline]
    pub fn path_rect(&mut self, rect: &Rect<f32>) {
        self.path.rect(rect)
    }

    #[inline]
    pub fn path_round_rect(&mut self, rect: &Rect<f32>, corners: &Corners<f32>) {
        self.path.round_rect(rect, corners);
    }

    #[inline]
    pub fn path_circle(&mut self, center: Vec2<f32>, radius: f32) {
        self.path.circle(center, radius);
    }

    #[inline]
    pub fn path_arc(
        &mut self,
        center: Vec2<f32>,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        clockwise: bool,
    ) {
        self.path
            .arc(center, radius, start_angle, end_angle, clockwise);
    }

    /// Fills the current path
    pub fn fill_path(&mut self, color: Color) {
        // FIXME:  add earcut("Fill Path")
        self.fill_path_convex(color, false);
    }

    pub(crate) fn fill_path_convex(&mut self, color: Color, calc_uv: bool) {
        // FIXME: move to drawlist;
        const FEATHERING: f32 = 1.0;
        let feathering = FEATHERING;
        let points_count = self.path.points.len();

        if points_count <= 2 || color.is_transparent() {
            return;
        }

        let bounds = if calc_uv {
            self.path.bounds()
        } else {
            Default::default()
        };

        let min = bounds.min();
        let max = bounds.max();

        let get_uv = |point: &Vec2<f32>| {
            if calc_uv {
                let uv_x = (point.x - min.x) / (max.x - min.x);
                let uv_y = (point.y - min.y) / (max.y - min.y);
                (uv_x, uv_y)
            } else {
                WHITE_UV
            }
        };

        if feathering > 0.0 {
            // AA fill
            let idx_count = (points_count - 2) * 3 + points_count * 6;
            let vtx_count = points_count * 2;
            self.mesh.reserve_prim(vtx_count, idx_count);

            let cur_vertex_idx = self.mesh.vertex_count();
            let vtx_inner_idx = cur_vertex_idx;
            let vtx_outer_idx = vtx_inner_idx + 1;
            for i in 2..points_count {
                self.mesh.add_triangle(
                    vtx_inner_idx,
                    vtx_inner_idx + ((i - 1) << 1) as u32,
                    vtx_inner_idx + (i << 1) as u32,
                );
            }

            let mut i0 = points_count - 1;
            for i1 in 0..points_count {
                let p1 = self.path.points[i1];
                let dm = p1.normalize().normal() * 0.5 * feathering;

                let pos_inner = p1 - dm;
                let pos_outer = p1 + dm;

                self.mesh.add_vertex(pos_inner, color, get_uv(&pos_inner));
                self.mesh.add_vertex(pos_outer, color, get_uv(&pos_outer));

                self.mesh.add_triangle(
                    vtx_inner_idx + (i1 << 1) as u32,
                    vtx_inner_idx + (i0 << 1) as u32,
                    vtx_outer_idx + (i0 << 1) as u32,
                );

                self.mesh.add_triangle(
                    vtx_outer_idx + (i0 << 1) as u32,
                    vtx_outer_idx + (i1 << 1) as u32,
                    vtx_inner_idx + (i1 << 1) as u32,
                );

                i0 = i1;
            }
        } else {
            // no AA fill
            let index_count = (points_count - 2) * 3;
            let vtx_count = points_count;

            self.mesh.reserve_prim(vtx_count, index_count);
            let base_idx = self.mesh.vertex_count();

            for point in &self.path.points {
                let uv = get_uv(point);
                self.mesh.add_vertex(*point, color, uv);
            }

            for i in 2..points_count {
                self.mesh.add_triangle(
                    base_idx,                //
                    base_idx + i as u32 - 1, //
                    base_idx + i as u32,     //
                );
            }
        }
    }

    /// Add stroke using the current path
    pub fn stroke_path(&mut self, stroke_style: &StrokeStyle) {
        if stroke_style.color.is_transparent() {
            return;
        }

        Polyline::add_to_mesh(&mut self.mesh, &self.path.points, stroke_style);
    }

    /// Add stroke using the given path
    pub fn stroke_with_path(&mut self, path: &Path2D, stroke_style: &StrokeStyle) {
        Polyline::add_to_mesh(&mut self.mesh, &path.points, stroke_style);
    }

    pub fn fill_with_path(&mut self, _path: &Path2D, _color: Color) {
        // TODO: earcut for user facing api
    }

    pub fn add_prim_quad(&mut self, rect: &Rect<f32>, color: Color) {
        if color.is_transparent() {
            return;
        }

        let v_index_offset = self.mesh.vertex_count();
        self.mesh.reserve_prim(4, 6);

        self.mesh.add_vertex(rect.top_left(), color, (0.0, 0.0)); // Top-left
        self.mesh.add_vertex(rect.top_right(), color, (1.0, 0.0)); // Top-right
        self.mesh.add_vertex(rect.bottom_left(), color, (0.0, 1.0)); // Bottom-left
        self.mesh.add_vertex(rect.bottom_right(), color, (1.0, 1.0)); // Bottom-right

        self.mesh
            .add_triangle(v_index_offset, v_index_offset + 1, v_index_offset + 2);

        self.mesh
            .add_triangle(v_index_offset + 2, v_index_offset + 1, v_index_offset + 3);
    }

    pub fn add_triangle_fan(
        &mut self,
        color: Color,
        connect_to: Vec2<f32>,
        origin: Vec2<f32>,
        start: Vec2<f32>,
        end: Vec2<f32>,
        clockwise: bool,
    ) {
        self.mesh
            .add_triangle_fan(color, connect_to, origin, start, end, clockwise);
    }

    pub fn build(mut self) -> Mesh {
        std::mem::take(&mut self.mesh)
    }
}

impl From<DrawList> for Mesh {
    #[inline]
    fn from(value: DrawList) -> Self {
        value.build()
    }
}

/*
    -------------- edge_1 ---------| | |
    -------------- center ---------| | |
    -------------- edge_2 ---------| | |
                                   | | |
*/

#[derive(Debug, Clone)]
pub struct PolySegment {
    pub edge1: LineSegment,
    pub center: LineSegment,
    pub edge2: LineSegment,
}

impl PolySegment {
    pub fn new(center: LineSegment, line_width: f32) -> Self {
        let normal = center.normal();
        let edge_1 = center.clone() + normal * line_width;
        let edge_2 = center.clone() - (normal * line_width);

        Self {
            center,
            edge1: edge_1,
            edge2: edge_2,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LineSegment {
    pub a: Vec2<f32>,
    pub b: Vec2<f32>,
}

impl LineSegment {
    pub fn new(a: Vec2<f32>, b: Vec2<f32>) -> Self {
        Self { a, b }
    }
}

impl std::ops::Add<Vec2<f32>> for LineSegment {
    type Output = Self;
    fn add(self, other: Vec2<f32>) -> Self::Output {
        Self {
            a: self.a + other,
            b: self.b + other,
        }
    }
}

impl std::ops::Mul<f32> for LineSegment {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self::Output {
        Self {
            a: self.a * scalar,
            b: self.b + scalar,
        }
    }
}

impl std::ops::Sub<Vec2<f32>> for LineSegment {
    type Output = Self;
    fn sub(self, other: Vec2<f32>) -> Self::Output {
        Self {
            a: self.a - other,
            b: self.b - other,
        }
    }
}

impl LineSegment {
    pub fn direction(&self) -> Vec2<f32> {
        self.a.direction(self.b)
    }

    pub fn direction_unnormalized(&self) -> Vec2<f32> {
        self.a - self.b
    }

    pub fn normal(&self) -> Vec2<f32> {
        Vec2::new(-(self.b.y - self.a.y), self.b.x - self.a.x).direction(Vec2::new(0.0, 0.0))
    }

    // https://www.desmos.com/calculator/ujamclid3g
    pub fn intersection(
        &self,
        other: &LineSegment,
        allow_infinite_lines: bool,
    ) -> Option<Vec2<f32>> {
        let dir_self = self.b - self.a;
        let dir_other = other.b - other.a;

        let origin_dist = other.a - self.a;
        let numerator = origin_dist.cross(&dir_self);
        let denominator = dir_self.cross(&dir_other);

        // parallel
        if denominator.abs() < 0.0001 {
            return None;
        }
        let u = numerator / denominator;
        let t = origin_dist.cross(&dir_other) / denominator;

        if !allow_infinite_lines && (!(0.0..=1.0).contains(&t) || !(0.0..=1.0).contains(&u)) {
            return None;
        }

        Some(self.a + dir_self * t)
    }
}
