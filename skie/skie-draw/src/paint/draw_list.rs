use std::fmt::Debug;

use super::path::GeometryPath;
use super::{Color, LineCap, Rgba, StrokeStyle, TextureId};

use crate::math::{Corners, Rect, Vec2};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct DrawVert {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    // FIXME: use u32 maybe?
    pub color: Rgba,
}

impl DrawVert {
    pub fn new(pos: Vec2<f32>, color: impl Into<Rgba>, uv: (f32, f32)) -> Self {
        Self {
            position: pos.into(),
            uv: uv.into(),
            color: color.into(),
        }
    }
}

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<DrawVert>,
    pub indices: Vec<u32>,
    pub texture: TextureId,
}

pub type DrawListMiddleware<'a> = Box<dyn Fn(DrawVert) -> DrawVert + 'a>;

#[derive(Default)]
pub struct DrawList<'a> {
    pub(crate) vertices: Vec<DrawVert>,
    pub(crate) indices: Vec<u32>,
    pub(crate) path: GeometryPath,
    cur_vertex_idx: u32,

    middleware: Option<DrawListMiddleware<'a>>,
}

impl<'a> DrawList<'a> {
    pub fn with_middleware<F>(middleware: F) -> Self
    where
        F: Fn(DrawVert) -> DrawVert + 'a,
    {
        Self {
            middleware: Some(Box::new(middleware)),
            ..Default::default()
        }
    }

    pub fn set_middleware<F>(&mut self, middleware: F)
    where
        F: Fn(DrawVert) -> DrawVert + 'a,
    {
        self.middleware = Some(Box::new(middleware));
    }

    pub fn set_no_middleware(&mut self) {
        self.middleware = None;
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.path.clear();
        self.cur_vertex_idx = 0;
    }

    #[inline]
    pub fn apply_mw(&self, vertex: DrawVert) -> DrawVert {
        let mut vertex = vertex;
        if let Some(middleware) = &self.middleware {
            vertex = middleware(vertex);
        }
        vertex
    }

    pub(crate) fn fill_path_convex(&mut self, color: Color) {
        let points_count = self.path.points.len();

        if points_count <= 2 {
            return;
        }

        let index_count = (points_count - 2) * 3;
        let vtx_count = points_count;

        self.reserve_prim(vtx_count, index_count);

        let bounds = self.path.bounds();
        let min_x = bounds.x;
        let min_y = bounds.y;
        let max_x = bounds.x + bounds.width;
        let max_y = bounds.y + bounds.height;

        for point in &self.path.points {
            let uv_x = (point.x - min_x) / (max_x - min_x);
            let uv_y = (point.y - min_y) / (max_y - min_y);
            let uv = (uv_x, uv_y);

            self.vertices
                .push(self.apply_mw(DrawVert::new(*point, color, uv)));
        }

        let base_idx = self.cur_vertex_idx;

        // We assume the first vertex (base) is the center of the fan
        for i in 2..points_count {
            let idx0 = base_idx; // First vertex (center of the fan)
            let idx1 = base_idx + i as u32 - 1; // The second vertex in the fan
            let idx2 = base_idx + i as u32; // The next vertex in the fan

            self.indices.push(idx0);
            self.indices.push(idx1);
            self.indices.push(idx2);
        }

        self.cur_vertex_idx += vtx_count as u32;
    }

    #[inline]
    pub fn begin_path(&mut self) {
        self.path.clear();
    }

    #[inline]
    pub fn close_path(&mut self) {
        self.path.close_path();
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
        self.path
            .arc(center, radius, 0.0, std::f32::consts::TAU, false);
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
    pub fn fill_path(&mut self) {
        todo!("Fill Path")
    }

    /// Strokes the current path
    pub fn stroke_path(&mut self, stroke_style: &StrokeStyle) {
        // FIXME: is this good
        let points: Vec<Vec2<f32>> = std::mem::take(&mut self.path.points);
        self.add_polyline(&points, stroke_style);
        self.path.points = points;
    }

    pub fn fill_with_path(&mut self, _path: &GeometryPath) {
        todo!()
    }

    pub fn stroke_with_path(&mut self, path: &GeometryPath, stroke_style: &StrokeStyle) {
        self.add_polyline(&path.points, stroke_style)
    }

    fn add_polyline(&self, points: &[Vec2<f32>], stroke_style: &StrokeStyle) {
        if points.len() < 2 {
            return;
        }

        let h_linewidth = stroke_style.line_width as f32 / 2.0;

        let mut segments: Vec<PolySegment> = points
            .windows(2)
            .filter(|p| p[0] != p[1])
            .map(|p| PolySegment::new(LineSegment::new(p[0], p[1]), h_linewidth))
            .collect();

        if stroke_style.line_cap == LineCap::Joint && points.first() != points.last() {
            segments.push(PolySegment::new(
                LineSegment::new(*points.last().unwrap(), *points.first().unwrap()),
                h_linewidth,
            ));
        }

        if segments.is_empty() {
            return;
        }

        {
            let mut out = String::new();
            out.push('[');

            let mut push_vec = |l: &LineSegment| {
                out.push_str(&format!("({},{}), ({}, {}),", l.a.x, l.a.y, l.b.x, l.b.y));
            };

            for segment in &segments {
                push_vec(&segment.edge_1);
                push_vec(&segment.center);
                push_vec(&segment.edge_2);
            }

            out = out.trim_end_matches(',').to_string();
            out.push(']');

            println!("{}", out);
        }
    }

    pub fn reserve_prim(&mut self, vertex_count: usize, index_count: usize) {
        self.vertices.reserve(vertex_count);
        self.indices.reserve(index_count);
    }

    #[inline]
    pub fn add_vertex(&mut self, pos: Vec2<f32>, color: Color, uv: (f32, f32)) {
        self.vertices
            .push(self.apply_mw(DrawVert::new(pos, color, uv))); // Top-left
    }

    pub fn add_prim_quad(&mut self, rect: &Rect<f32>, color: Color) {
        let v_index_offset = self.cur_vertex_idx;

        let Rect {
            x,
            y,
            width,
            height,
        } = *rect;

        let uvs: [(f32, f32); 4] = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];

        self.reserve_prim(4, 6);

        self.add_vertex((x, y).into(), color, uvs[0]); // Top-left
        self.add_vertex((x + width, y).into(), color, uvs[1]); // Top-right
        self.add_vertex((x, y + height).into(), color, uvs[2]); // Bottom-left
        self.add_vertex((x + width, y + height).into(), color, uvs[3]); // Bottom-right

        self.indices.extend_from_slice(&[
            v_index_offset,
            v_index_offset + 1,
            v_index_offset + 2,
            v_index_offset + 2,
            v_index_offset + 1,
            v_index_offset + 3,
        ]);

        self.cur_vertex_idx += 4;
    }

    pub fn build(mut self, texture: TextureId) -> Mesh {
        let vertices = std::mem::take(&mut self.vertices);
        let indices = std::mem::take(&mut self.indices);

        Mesh {
            vertices,
            indices,
            texture,
        }
    }
}

impl From<DrawList<'_>> for Mesh {
    #[inline]
    fn from(value: DrawList<'_>) -> Self {
        value.build(TextureId::WHITE_TEXTURE)
    }
}

impl std::fmt::Display for DrawList<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DrawList")
            .field("vertices", &self.vertices)
            .field("indices", &self.indices)
            .field("indices", &self.cur_vertex_idx)
            .field("has_middleware", &format!("{}", self.middleware.is_some()))
            .finish()
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
    pub edge_1: LineSegment,
    pub center: LineSegment,
    pub edge_2: LineSegment,
}

impl PolySegment {
    pub fn new(center: LineSegment, line_width: f32) -> Self {
        let normal = center.normal();
        let edge_1 = center.clone() + normal * line_width;
        let edge_2 = center.clone() - (normal * line_width);

        Self {
            center,
            edge_1,
            edge_2,
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
