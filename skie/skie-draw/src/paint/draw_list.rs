use core::f32;
use std::ops::Range;

use skie_math::IsZero;

use super::{
    Brush, Circle, Color, FillStyle, Mesh, PathBrush, Primitive, Quad, StrokeTesellator, Vertex,
};

use crate::earcut::Earcut;
use crate::math::{Rect, Vec2};
use crate::paint::WHITE_UV;
use crate::{get_path_bounds, PathEventsIter, PathGeometryBuilder};

use std::ops::{Deref, DerefMut};

use crate::path::{Path, PathBuilder, Point};

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
    pub(crate) feathering: f32,
    pub(crate) mesh: Mesh,
    pub(crate) temp_path: ScratchPathBuilder,
    pub(crate) temp_path_data: Vec<Point>,
    earcut: Earcut<f32>,
}

impl DrawList {
    pub fn feathering(&mut self, value: f32) -> f32 {
        let old = self.feathering;
        self.feathering = value;
        old
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
        let stroke_color = brush.stroke_style.color;

        self.temp_path.clear();
        self.temp_path_data.clear();

        if quad.corners.is_zero() {
            self.temp_path.rect(&quad.bounds);
        } else {
            self.temp_path.round_rect(&quad.bounds, &quad.corners);
        }

        build_path_single_contour(
            self.temp_path.path_events(),
            &mut self.temp_path_data,
            |path| {
                fill_path_convex(
                    &mut self.mesh,
                    path,
                    fill_color,
                    textured,
                    brush.feathering,
                    (!stroke_color.is_transparent()).then_some(stroke_color),
                );
                StrokeTesellator::add_to_mesh(&mut self.mesh, path, &brush.stroke_style);
            },
        );
    }

    pub fn add_circle(&mut self, circle: &Circle, brush: &Brush, textured: bool) {
        let fill_color = brush.fill_style.color;
        let stroke_color = brush.stroke_style.color;

        self.temp_path.clear();
        self.temp_path.circle(circle.center, circle.radius);

        self.temp_path_data.clear();

        build_path_single_contour(
            self.temp_path.path_events(),
            &mut self.temp_path_data,
            |path| {
                fill_path_convex(
                    &mut self.mesh,
                    path,
                    fill_color,
                    textured,
                    brush.feathering,
                    (!stroke_color.is_transparent()).then_some(stroke_color),
                );
                StrokeTesellator::add_to_mesh(&mut self.mesh, path, &brush.stroke_style);
            },
        );
    }

    pub fn add_path(&mut self, path: &Path, brush: &PathBrush) {
        self.temp_path_data.clear();
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

#[inline]
pub fn build_path(
    iter: PathEventsIter,
    output: &mut Vec<Point>,
    brush: &PathBrush,
    mut f: impl FnMut(&Brush, &[Point]),
) {
    let geo_build = <PathGeometryBuilder<PathEventsIter>>::new(iter, output).collect::<Vec<_>>();

    for (contour, range) in geo_build {
        let this_brush = brush.get_or_default(&contour);
        f(&this_brush, &output[range.clone()])
    }
}

#[inline]
pub fn build_path_single_contour(
    iter: PathEventsIter,
    output: &mut Vec<Point>,
    mut f: impl FnMut(&[Point]),
) {
    if let Some((_, range)) = <PathGeometryBuilder<PathEventsIter>>::new(iter, output).next() {
        f(&output[range])
    } else {
        log::warn!("build_path_single_contour called with path with no contour!");
    }
}

fn fill_path_convex(
    mesh: &mut Mesh,
    path: &[Point],
    fill: Color,
    textured: bool,
    _feathering: f32,
    _fade_to: Option<Color>,
) {
    let points_count = path.len() as u32;

    if points_count < 3 || fill.is_transparent() {
        return;
    }

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

    let index_count = (points_count - 2) * 3;
    let vtx_count = points_count;

    mesh.reserve_prim(vtx_count as usize, index_count as usize);
    let base_idx = mesh.vertex_count();

    for point in path {
        let uv = get_uv(point);
        mesh.add_vertex(*point, fill, uv);
    }

    for i in 2..points_count {
        mesh.add_triangle(
            base_idx,         //
            base_idx + i - 1, //
            base_idx + i,     //
        );
    }
}
