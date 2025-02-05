use core::f32;
use std::ops::Range;

use skie_math::IsZero;

use super::{
    Brush, Circle, Color, FillStyle, Mesh, PathBrush, Primitive, Quad, StrokeTesellator, Vertex,
};

use crate::earcut::Earcut;
use crate::math::{Rect, Vec2};
use crate::paint::WHITE_UV;

use std::ops::{Deref, DerefMut};

use crate::path::{Path, PathBuilder, PathEventsIter, PathGeometryBuilder, Point};

#[derive(Default)]
pub struct ScratchPathBuilder(PathBuilder);

impl ScratchPathBuilder {
    #[inline(always)]
    fn clear(&mut self) {
        self.points.clear();
        self.verbs.clear();
    }
}

impl Deref for ScratchPathBuilder {
    type Target = PathBuilder;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ScratchPathBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Default)]
pub struct DrawList {
    pub(crate) antialias: bool,
    pub(crate) feathering: f32,
    pub(crate) mesh: Mesh,
    pub(crate) temp_path: ScratchPathBuilder,
    pub(crate) temp_path_data: Vec<Point>,
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
        self.temp_path.clear();
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

    pub fn add_quad(&mut self, quad: &Quad, brush: &Brush, textured: bool) {
        let fill_color = brush.fill_style.color;

        self.temp_path.clear();

        if quad.corners.is_zero() {
            self.temp_path.rect(&quad.bounds);
        } else {
            self.temp_path.round_rect(&quad.bounds, &quad.corners);
        }

        build_path(
            self.temp_path.path_events(),
            &mut self.temp_path_data,
            &PathBrush::new(brush.clone()),
            |_, points| {
                fill_path_convex(
                    &mut self.mesh,
                    points,
                    self.feathering,
                    fill_color,
                    textured,
                );

                StrokeTesellator::add_to_mesh(&mut self.mesh, points, &brush.stroke_style);
            },
        );
    }

    pub fn add_circle(&mut self, circle: &Circle, brush: &Brush, textured: bool) {
        let fill_color = brush.fill_style.color;

        self.temp_path.clear();
        self.temp_path.circle(circle.center, circle.radius);

        build_path(
            self.temp_path.path_events(),
            &mut self.temp_path_data,
            &PathBrush::new(brush.clone()),
            |_, points| {
                fill_path_convex(
                    &mut self.mesh,
                    points,
                    self.feathering,
                    fill_color,
                    textured,
                );

                StrokeTesellator::add_to_mesh(&mut self.mesh, points, &brush.stroke_style);
            },
        );
    }

    pub fn add_path(&mut self, path: &Path, brush: &PathBrush) {
        build_path(
            path.events(),
            &mut self.temp_path_data,
            brush,
            |brush, points| {
                Self::fill_earcut(points, &mut self.mesh, &mut self.earcut, &brush.fill_style);
                StrokeTesellator::add_to_mesh(&mut self.mesh, points, &brush.stroke_style);
            },
        );
    }

    pub fn add_primitive(&mut self, primitive: &Primitive, brush: &Brush, textured: bool) {
        match primitive {
            Primitive::Circle(circle) => self.add_circle(circle, brush, textured),

            Primitive::Quad(quad) => self.add_quad(quad, brush, textured),

            Primitive::Path { path, brush } => self.add_path(path, brush),
        };
    }

    fn fill_earcut(
        points: &[Vec2<f32>],
        mesh: &mut Mesh,
        earcut: &mut Earcut<f32>,
        fill_style: &FillStyle,
    ) {
        // TODO: AA fill
        // TODO: support holes ?

        if fill_style.color.is_transparent() {
            return;
        }

        let vertex_offset = mesh.vertices.len() as u32;
        let index_offset = mesh.indices.len();

        earcut.earcut(
            points.iter().map(|p| [p.x, p.y]),
            &[],
            &mut mesh.indices,
            false,
        );

        if index_offset == mesh.indices.len() {
            return;
        }

        // indices are reserved by earcut
        mesh.vertices.reserve(points.len());

        for point in points {
            mesh.add_vertex(*point, fill_style.color, WHITE_UV);
        }

        for i in &mut mesh.indices[index_offset..] {
            *i += vertex_offset;
        }
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

fn fill_path_convex(
    mesh: &mut Mesh,
    path: &[Point],
    feathering: f32,
    color: Color,
    textured: bool,
) {
    let points_count = path.len();

    if points_count <= 2 || color.is_transparent() {
        return;
    }
    let mut color_out = color;
    color_out.a = 0;

    let bounds = if textured {
        get_path_bounds(path)
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
    if feathering > 0.0 {
        // AA fill
        let idx_count = (points_count - 2) * 3 + points_count * 6;
        let vtx_count = points_count * 2;
        mesh.reserve_prim(vtx_count, idx_count);

        let cur_vertex_idx = mesh.vertex_count();
        let vtx_inner_idx = cur_vertex_idx;
        let vtx_outer_idx = vtx_inner_idx + 1;
        for i in 2..points_count {
            mesh.add_triangle(
                vtx_inner_idx,
                vtx_inner_idx + ((i - 1) << 1) as u32,
                vtx_inner_idx + (i << 1) as u32,
            );
        }

        let mut i0 = points_count - 1;
        for i1 in 0..points_count {
            let p1 = path[i1];
            let dm = p1.normalize().normal() * 0.5 * feathering;

            let pos_inner = p1 - dm;
            let pos_outer = p1 + dm;

            mesh.add_vertex(pos_inner, color, get_uv(&pos_inner));
            mesh.add_vertex(pos_outer, color_out, get_uv(&pos_outer));

            mesh.add_triangle(
                vtx_inner_idx + (i1 << 1) as u32,
                vtx_inner_idx + (i0 << 1) as u32,
                vtx_outer_idx + (i0 << 1) as u32,
            );

            mesh.add_triangle(
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

        mesh.reserve_prim(vtx_count, index_count);
        let base_idx = mesh.vertex_count();

        for point in path {
            let uv = get_uv(point);
            mesh.add_vertex(*point, color, uv);
        }

        for i in 2..points_count {
            mesh.add_triangle(
                base_idx,                //
                base_idx + i as u32 - 1, //
                base_idx + i as u32,     //
            );
        }
    }
}

fn build_path(
    iter: PathEventsIter,
    output: &mut Vec<Point>,
    brush: &PathBrush,
    mut f: impl FnMut(&Brush, &[Point]),
) {
    let geo_build =
        <PathGeometryBuilder<PathEventsIter>>::new(iter, output, true).collect::<Vec<_>>();

    for (contour, range) in geo_build {
        let this_brush = brush.get_or_default(&contour);
        f(&this_brush, &output[range.clone()])
    }
}

fn get_path_bounds(path: &[Point]) -> Rect<f32> {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;

    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for point in path {
        let x = point.x;
        let y = point.y;
        min_x = if x < min_x { x } else { min_x };
        max_x = if x > max_x { x } else { max_x };

        min_y = if y < min_y { y } else { min_y };
        max_y = if y > max_y { y } else { max_y };
    }

    Rect::from_corners((min_x, min_y).into(), (max_x, max_y).into())
}
