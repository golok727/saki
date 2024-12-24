use core::f32;
use std::fmt::Debug;

use super::path::Path2D;
use super::{Color, LineCap, LineJoin, Rgba, StrokeStyle, TextureId};

use crate::math::{Corners, Rect, Vec2};
use crate::paint::WHITE_TEXTURE_UV;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct DrawVert {
    pub position: [f32; 2],
    pub uv: [f32; 2],
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
    pub(crate) path: Path2D,
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

        if points_count <= 2 || color.is_transparent() {
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
    pub fn fill_path(&mut self, color: Color) {
        // FIXME:  add earcut("Fill Path")
        self.fill_path_convex(color);
    }

    /// Strokes the current path
    pub fn stroke_path(&mut self, stroke_style: &StrokeStyle) {
        if stroke_style.color.is_transparent() {
            return;
        }

        // FIXME: is this good
        let points: Vec<Vec2<f32>> = std::mem::take(&mut self.path.points);
        self.add_polyline(&points, stroke_style);
        self.path.points = points;
    }

    pub fn fill_with_path(&mut self, _path: &Path2D, _color: Color) {
        // TODO: earcut for user facing api
    }

    pub fn stroke_with_path(&mut self, path: &Path2D, stroke_style: &StrokeStyle) {
        self.add_polyline(&path.points, stroke_style)
    }

    // Adapted from
    // https://github.com/CrushedPixel/Polyline2D
    // https://artgrammer.blogspot.com/2011/07/drawing-polylines-by-tessellation.html?m=1
    fn add_polyline(&mut self, points: &[Vec2<f32>], stroke_style: &StrokeStyle) {
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

        let first_segment = segments.first().unwrap();
        let last_segment = segments.last().unwrap();

        // Path start and end edge vertices
        let mut path_start_1 = first_segment.edge1.a;
        let mut path_start_2 = first_segment.edge2.a;

        let mut path_end_1 = last_segment.edge1.b;
        let mut path_end_2 = last_segment.edge2.b;

        match stroke_style.line_cap {
            LineCap::Butt => {
                // NOOP
            }
            LineCap::Round => {
                // add the start and end round caps
                self.add_triangle_fan(
                    stroke_style.color,
                    first_segment.center.a,
                    first_segment.center.a,
                    path_start_1,
                    path_start_2,
                    false,
                );

                self.add_triangle_fan(
                    stroke_style.color,
                    last_segment.center.b,
                    last_segment.center.b,
                    path_end_1,
                    path_end_2,
                    true,
                );
            }
            LineCap::Joint => self.polyline_create_joint(
                stroke_style,
                last_segment,
                first_segment,
                &mut path_end_1,
                &mut path_end_2,
                &mut path_start_1,
                &mut path_start_2,
            ),
            LineCap::Square => {
                // offset the start and end with the half line width
                path_start_1 += first_segment.edge1.direction() * h_linewidth;
                path_start_2 += first_segment.edge2.direction() * h_linewidth;
                path_end_1 -= last_segment.edge1.direction() * h_linewidth;
                path_end_2 -= last_segment.edge2.direction() * h_linewidth;
            }
        }

        let mut start_1: Vec2<f32> = Vec2::default();
        let mut start_2: Vec2<f32> = Vec2::default();

        let mut next_start_1: Vec2<f32> = Vec2::default();
        let mut next_start_2: Vec2<f32> = Vec2::default();

        let mut end_1: Vec2<f32> = Vec2::default();
        let mut end_2: Vec2<f32> = Vec2::default();

        for (i, segment) in segments.iter().enumerate() {
            if i == 0 {
                start_1 = path_start_1;
                start_2 = path_start_2;
            }

            if i + 1 == segments.len() {
                end_1 = path_end_1;
                end_2 = path_end_2;
            } else {
                // join the two segments
                self.polyline_create_joint(
                    stroke_style,
                    segment,
                    &segments[i + 1],
                    &mut end_1,
                    &mut end_2,
                    &mut next_start_1,
                    &mut next_start_2,
                )
            }

            let cur_vertex_idx = self.cur_vertex_idx;

            // emit vertices
            self.reserve_prim(4, 6);
            self.add_vertex(start_1, stroke_style.color, (0.0, 0.0));
            self.add_vertex(start_2, stroke_style.color, (0.0, 0.0));
            self.add_vertex(end_1, stroke_style.color, (0.0, 0.0));
            self.add_vertex(end_2, stroke_style.color, (0.0, 0.0));
            self.indices.extend_from_slice(&[
                cur_vertex_idx,
                cur_vertex_idx + 1,
                cur_vertex_idx + 2,
                cur_vertex_idx + 2,
                cur_vertex_idx + 1,
                cur_vertex_idx + 3,
            ]);
            self.cur_vertex_idx += 4;

            start_1 = next_start_1;
            start_2 = next_start_2;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn polyline_create_joint(
        &mut self,

        style: &StrokeStyle,
        segment1: &PolySegment,
        segment2: &PolySegment,

        end1: &mut Vec2<f32>,
        end2: &mut Vec2<f32>,

        next_start1: &mut Vec2<f32>,
        next_start2: &mut Vec2<f32>,
    ) {
        let dir1 = segment1.center.direction();
        let dir2 = segment2.center.direction();

        let angle = dir1.angle(&dir2);

        let mut wrapped_angle = angle;
        if wrapped_angle > f32::consts::FRAC_PI_2 {
            wrapped_angle = f32::consts::PI - wrapped_angle;
        }

        const MITER_MIN_ANGLE: f32 = 0.349066; // ~20 degrees
        let mut joint_style = style.line_join;

        if joint_style == LineJoin::Miter && wrapped_angle < MITER_MIN_ANGLE {
            joint_style = LineJoin::Bevel;
        }

        if joint_style == LineJoin::Miter {
            // calculate each edge's intersection point
            // with the next segment's central line
            let sec1 = segment1.edge1.intersection(&segment2.edge1, true);
            let sec2 = segment1.edge2.intersection(&segment2.edge2, true);

            *end1 = sec1.unwrap_or(segment1.edge1.b);
            *end2 = sec2.unwrap_or(segment1.edge2.b);

            *next_start1 = *end1;
            *next_start2 = *end2;
        } else {
            let x1 = dir1.x;
            let x2 = dir2.x;
            let y1 = dir1.y;
            let y2 = dir2.y;

            let clockwise = x1 * y2 - x2 * y1 < 0.;

            let inner1: &LineSegment;
            let inner2: &LineSegment;
            let outer1: &LineSegment;
            let outer2: &LineSegment;

            if clockwise {
                outer1 = &segment1.edge1;
                outer2 = &segment2.edge1;
                inner1 = &segment1.edge2;
                inner2 = &segment2.edge2;
            } else {
                outer1 = &segment1.edge2;
                outer2 = &segment2.edge2;
                inner1 = &segment1.edge1;
                inner2 = &segment2.edge1;
            }

            let inner_sec_option = inner1.intersection(inner2, style.allow_overlap);
            let inner_sec = inner_sec_option.unwrap_or(inner1.b);

            let inner_start = if inner_sec_option.is_some() {
                inner_sec
            } else if angle > f32::consts::FRAC_PI_2 {
                outer1.b
            } else {
                inner1.b
            };

            if clockwise {
                *end1 = outer1.b;
                *end2 = inner_sec;

                *next_start1 = outer2.a;
                *next_start2 = inner_start;
            } else {
                *end1 = inner_sec;
                *end2 = outer1.b;

                *next_start1 = inner_start;
                *next_start2 = outer2.a;
            }

            if joint_style == LineJoin::Bevel {
                // simply connect the intersection points
                self.reserve_prim(3, 3);
                self.add_vertex(outer1.b, style.color, (0.0, 0.0));
                self.add_vertex(outer2.a, style.color, (0.0, 0.0));
                self.add_vertex(inner_sec, style.color, (0.0, 0.0));
                self.indices.extend_from_slice(&[
                    self.cur_vertex_idx,
                    self.cur_vertex_idx + 1,
                    self.cur_vertex_idx + 2,
                ]);
                self.cur_vertex_idx += 3;
            } else if joint_style == LineJoin::Round {
                self.add_triangle_fan(
                    style.color,
                    inner_sec,
                    segment1.center.b,
                    outer1.b,
                    outer2.a,
                    clockwise,
                );
            }
        }
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
        let from = start - origin;
        let to = end - origin;

        let mut from_angle = from.y.atan2(from.x);
        let mut to_angle = to.y.atan2(to.x);

        if clockwise {
            if to_angle > from_angle {
                to_angle -= f32::consts::TAU;
            }
        } else if from_angle > to_angle {
            from_angle -= f32::consts::TAU;
        }

        const ROUND_MIN_ANGLE: f32 = 0.174533; // ~10 deg

        let angle = to_angle - from_angle;
        let num_triangles = (angle / ROUND_MIN_ANGLE).abs().floor().max(1.0) as usize;
        let seg_angle = angle / num_triangles as f32;

        self.add_vertex(connect_to, color, WHITE_TEXTURE_UV);
        self.add_vertex(start, color, WHITE_TEXTURE_UV);

        let conn_vertex_index = self.cur_vertex_idx;
        let start_vertex_index = self.cur_vertex_idx + 1;
        self.cur_vertex_idx += 2;

        let mut prev_vertex_index = start_vertex_index;

        for i in 0..num_triangles - 1 {
            let rotation = (i as f32 + 1.0) * seg_angle;
            let end_point = Vec2::new(
                rotation.cos() * from.x - rotation.sin() * from.y,
                rotation.sin() * from.x + rotation.cos() * from.y,
            ) + origin;

            self.add_vertex(end_point, color, WHITE_TEXTURE_UV);
            self.indices.extend_from_slice(&[
                conn_vertex_index,
                prev_vertex_index,
                self.cur_vertex_idx,
            ]);
            prev_vertex_index = self.cur_vertex_idx;
            self.cur_vertex_idx += 1;
        }

        // add the end point avoids redudent end point calculation
        self.add_vertex(end, color, WHITE_TEXTURE_UV);
        self.indices.extend_from_slice(&[
            conn_vertex_index,
            prev_vertex_index,
            self.cur_vertex_idx,
        ]);
        self.cur_vertex_idx += 1;
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
        if color.is_transparent() {
            return;
        }
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

    pub fn build(mut self) -> Mesh {
        let vertices = std::mem::take(&mut self.vertices);

        let indices = std::mem::take(&mut self.indices);

        Mesh {
            vertices,
            indices,
            texture: TextureId::WHITE_TEXTURE,
        }
    }
}

impl From<DrawList<'_>> for Mesh {
    #[inline]
    fn from(value: DrawList<'_>) -> Self {
        value.build()
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
