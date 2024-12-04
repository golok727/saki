use super::{Quad, TextureId, DEFAULT_UV_COORD};

use crate::math::{Rect, Vec2};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
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
    // TODO default to white texture instead of optional
    pub texture: Option<TextureId>,
}

impl From<DrawList> for Mesh {
    fn from(mut dl: DrawList) -> Self {
        let vertices: Vec<Vertex> = std::mem::take(&mut dl.vertices);
        let indices: Vec<u32> = std::mem::take(&mut dl.indices);

        Self {
            vertices,
            indices,
            texture: None,
        }
    }
}

pub type DrawListMiddleware = Box<dyn Fn(Vertex) -> Vertex>;

#[derive(Default)]
pub struct DrawList {
    pub vertices: Vec<Vertex>,

    pub indices: Vec<u32>,

    // TODO may be allow only one middleware
    middlewares: Vec<DrawListMiddleware>,

    index_offset: u32,
}

impl DrawList {
    pub fn with_middlewares(middlewares: impl IntoIterator<Item = DrawListMiddleware>) -> Self {
        let middlewares: Vec<DrawListMiddleware> =
            middlewares.into_iter().map(Into::into).collect();

        Self {
            middlewares,
            ..Default::default()
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.index_offset = 0;
    }

    #[inline]
    pub fn apply_mw(&self, vertex: Vertex) -> Vertex {
        let mut vertex = vertex;
        for middleware in &self.middlewares {
            vertex = middleware(vertex);
        }
        vertex
    }

    pub fn push_quad(&mut self, quad: &Quad, has_texture: bool) {
        let index_offset = self.index_offset;

        let Rect {
            x,
            y,
            width,
            height,
        } = quad.bounds;

        let uvs: [(f32, f32); 4] = if has_texture {
            [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
        } else {
            [
                DEFAULT_UV_COORD.into(),
                DEFAULT_UV_COORD.into(),
                DEFAULT_UV_COORD.into(),
                DEFAULT_UV_COORD.into(),
            ]
        };

        let color = quad.background_color;

        self.vertices
            .push(self.apply_mw(Vertex::new((x, y).into(), color, uvs[0]))); // Top-left

        self.vertices
            .push(self.apply_mw(Vertex::new((x + width, y).into(), color, uvs[1]))); // Top-right
                                                                                     //
        self.vertices
            .push(self.apply_mw(Vertex::new((x, y + height).into(), color, uvs[2]))); // Bottom-left
                                                                                      //
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
}

impl std::fmt::Display for DrawList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DrawList")
            .field("vertices", &self.vertices)
            .field("indices", &self.indices)
            .field("indices", &self.index_offset)
            .field("middlewares", &format!("n = {}", self.middlewares.len()))
            .finish()
    }
}
