use core::f32;
use std::ops::Range;

use super::path::Path2D;
use super::{
    Brush, Circle, Color, FillStyle, Mesh, Polyline, Primitive, Quad, StrokeStyle, Vertex,
};

use crate::earcut::Earcut;
use crate::math::{Corners, Rect, Vec2};
use crate::paint::WHITE_UV;

#[derive(Default)]
pub struct DrawList {
    pub(crate) antialias: bool,
    pub(crate) feathering: f32,
    pub(crate) mesh: Mesh,
    pub(crate) path: Path2D,
    earcut: Earcut<f32>,
}

impl DrawList {
    pub fn is_antialiazed(&self) -> bool {
        self.antialias
    }

    pub fn antialias(&mut self, value: bool) {
        self.antialias = value
    }

    pub fn feathering(&mut self, value: f32) {
        self.feathering = value
    }

    pub fn clear(&mut self) {
        self.mesh.clear();
        self.path.clear();
    }

    /// captures any drawlist operations done inside the function `f` and returns a
    /// `DrawListCapture` allowing to modify the added vertex data
    pub fn capture(&mut self, f: impl FnOnce(&mut Self)) -> DrawListCapture<'_> {
        let start = self.mesh.vertices.len();
        f(self);
        let end = self.mesh.vertices.len();

        DrawListCapture {
            list: self,
            range: start..end,
        }
    }

    #[inline]
    pub fn capture_range(&mut self, f: impl FnOnce(&mut Self)) -> Range<usize> {
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

    pub fn add_quad(&mut self, quad: &Quad, brush: &Brush, textured: bool) {
        let fill_color = brush.fill_style.color;

        self.path.clear();
        self.path.round_rect(&quad.bounds, &quad.corners);
        self.fill_path_convex(fill_color, textured);
        self.stroke_path(&brush.stroke_style.join())
    }

    pub fn add_circle(&mut self, circle: &Circle, brush: &Brush, textured: bool) {
        let fill_color = brush.fill_style.color;

        self.path.clear();
        self.path.circle(circle.center, circle.radius);

        self.fill_path_convex(fill_color, textured);

        self.stroke_path(&brush.stroke_style.join())
    }

    pub fn add_path(&mut self, path: &Path2D, brush: &Brush) {
        self.fill_with_path(path, &brush.fill_style);

        let stroke_style = if path.closed {
            brush.stroke_style.join()
        } else {
            brush.stroke_style
        };

        self.stroke_with_path(path, &stroke_style);
    }

    pub fn add_primitive(&mut self, primitive: &Primitive, brush: &Brush, textured: bool) {
        match primitive {
            Primitive::Circle(circle) => self.add_circle(circle, brush, textured),

            Primitive::Quad(quad) => self.add_quad(quad, brush, textured),

            Primitive::Path(path) => self.add_path(path, brush),
        };
    }

    pub(crate) fn fill_path_convex(&mut self, color: Color, textured: bool) {
        let feathering = self.feathering;
        let points_count = self.path.points.len();

        if points_count <= 2 || color.is_transparent() {
            return;
        }
        let mut color_out = color;
        color_out.a = 0;

        let bounds = if textured {
            self.path.bounds()
        } else {
            Default::default()
        };

        let min = bounds.min();
        let max = bounds.max();

        let get_uv = |point: &Vec2<f32>| {
            if textured {
                let uv_x = (point.x - min.x) / (max.x - min.x);
                let uv_y = (point.y - min.y) / (max.y - min.y);
                (uv_x, uv_y)
            } else {
                WHITE_UV
            }
        };

        // FIXME:
        if self.antialias && self.feathering > 0.0 {
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
                self.mesh
                    .add_vertex(pos_outer, color_out, get_uv(&pos_outer));

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

    // fills the current path with the given fill_style
    pub fn fill_path(&mut self, fill_style: &FillStyle) {
        Self::fill_impl(
            &self.path.points,
            &mut self.mesh,
            &mut self.earcut,
            fill_style,
        );
    }

    pub fn fill_with_path(&mut self, path: &Path2D, fill_style: &FillStyle) {
        Self::fill_impl(&path.points, &mut self.mesh, &mut self.earcut, fill_style);
    }

    fn fill_impl(
        points: &[Vec2<f32>],
        mesh: &mut Mesh,
        earcut: &mut Earcut<f32>,
        fill_style: &FillStyle,
    ) {
        if fill_style.color.is_transparent() {
            return;
        }

        // TODO: AA fill
        let mut indices: Vec<u32> = vec![];
        // TODO: support holes ?
        earcut.earcut(points.iter().map(|p| [p.x, p.y]), &[], &mut indices);

        let offset = mesh.vertices.len() as u32;

        if indices.is_empty() {
            return;
        }

        for point in points {
            mesh.add_vertex(*point, fill_style.color, WHITE_UV);
        }

        mesh.reserve_prim(points.len(), indices.len());

        for i in &mut indices {
            *i += offset;
        }

        mesh.indices.extend(indices)
    }

    pub fn fill_rect(&mut self, rect: &Rect<f32>, color: Color) {
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

    pub fn build(&mut self) -> Mesh {
        std::mem::take(&mut self.mesh)
    }
}

pub struct DrawListCapture<'a> {
    list: &'a mut DrawList,
    range: Range<usize>,
}

impl<'a> DrawListCapture<'a> {
    pub fn map(self, f: impl Fn(&mut Vertex)) {
        self.list.map_range(self.range, f)
    }
}
