use super::{atlas::AtlasTextureId, Quad, TextureId};

use crate::math::{Rect, Vec2};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

fn wgpu_color_to_array(color: wgpu::Color) -> [f32; 4] {
    [
        color.r as f32,
        color.g as f32,
        color.b as f32,
        color.a as f32,
    ]
}

impl Vertex {
    pub fn new(pos: Vec2<f32>, color: wgpu::Color, uv: (f32, f32)) -> Self {
        Self {
            position: pos.into(),
            uv: uv.into(),
            color: wgpu_color_to_array(color),
        }
    }
}

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub texture: TextureId,
}

pub type DrawListMiddleware<'a> = Box<dyn Fn(Vertex) -> Vertex + 'a>;

#[derive(Default)]
pub struct DrawList<'a> {
    pub vertices: Vec<Vertex>,

    pub indices: Vec<u32>,

    middleware: Option<DrawListMiddleware<'a>>,

    index_offset: u32,
}

impl<'a> DrawList<'a> {
    pub fn with_middleware<F>(middleware: F) -> Self
    where
        F: Fn(Vertex) -> Vertex + 'a,
    {
        Self {
            middleware: Some(Box::new(middleware)),
            ..Default::default()
        }
    }

    pub fn set_middleware<F>(&mut self, middleware: F)
    where
        F: Fn(Vertex) -> Vertex + 'a,
    {
        self.middleware = Some(Box::new(middleware));
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.index_offset = 0;
    }

    #[inline]
    pub fn apply_mw(&self, vertex: Vertex) -> Vertex {
        let mut vertex = vertex;
        if let Some(middleware) = &self.middleware {
            vertex = middleware(vertex);
        }
        vertex
    }

    pub fn push_quad(&mut self, quad: &Quad) {
        let index_offset = self.index_offset;

        let Rect {
            x,
            y,
            width,
            height,
        } = quad.bounds;

        let uvs: [(f32, f32); 4] = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];

        let color = quad.background_color;

        self.vertices
            .push(self.apply_mw(Vertex::new((x, y).into(), color, uvs[0]))); // Top-left

        self.vertices
            .push(self.apply_mw(Vertex::new((x + width, y).into(), color, uvs[1]))); // Top-right

        self.vertices
            .push(self.apply_mw(Vertex::new((x, y + height).into(), color, uvs[2]))); // Bottom-left

        self.vertices.push(self.apply_mw(Vertex::new(
            (x + width, y + height).into(),
            color,
            uvs[3],
        ))); // Bottom-right

        self.indices.extend_from_slice(&[
            index_offset,
            index_offset + 1,
            index_offset + 2,
            index_offset + 2,
            index_offset + 1,
            index_offset + 3,
        ]);

        self.index_offset += 4;
    }

    pub fn build(mut self, texture: AtlasTextureId) -> Mesh {
        let vertices: Vec<Vertex> = std::mem::take(&mut self.vertices);
        let indices: Vec<u32> = std::mem::take(&mut self.indices);

        Mesh {
            vertices,
            indices,
            texture: TextureId::AtlasTexture(texture),
        }
    }
}

impl std::fmt::Display for DrawList<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DrawList")
            .field("vertices", &self.vertices)
            .field("indices", &self.indices)
            .field("indices", &self.index_offset)
            .field("has_middleware", &format!("{}", self.middleware.is_some()))
            .finish()
    }
}
