use crate::Vec2;

use super::{Rgba, TextureId};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: Rgba,
}

impl Vertex {
    pub fn new(pos: Vec2<f32>, color: impl Into<Rgba>, uv: (f32, f32)) -> Self {
        Self {
            position: pos.into(),
            uv: uv.into(),
            color: color.into(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub texture: TextureId,
}

impl Mesh {
    pub fn clear(&mut self) {
        self.indices.clear();
        self.vertices = Default::default();
    }

    #[inline(always)]
    pub fn add_vertex(&mut self, vertex: Vertex) {
        self.vertices.push(vertex)
    }

    pub fn append(&mut self, other: &Self) {
        debug_assert!(other.is_valid());

        let index_offset = self.vertices.len() as u32;
        self.indices
            .extend(other.indices.iter().map(|index| index + index_offset));
        self.vertices.extend(other.vertices.iter());
    }

    pub fn is_valid(&self) -> bool {
        if let Ok(n) = u32::try_from(self.vertices.len()) {
            self.indices.iter().all(|&i| i < n)
        } else {
            false
        }
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty() && self.vertices.is_empty()
    }

    #[inline(always)]
    pub fn reserve_prim(&mut self, vertex_count: usize, index_count: usize) {
        self.vertices.reserve(vertex_count);
        self.indices.reserve(index_count);
    }

    #[inline(always)]
    pub fn add_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.push(a);
        self.indices.push(b);
        self.indices.push(c);
    }

    #[inline(always)]
    pub fn vertex_count(&self) -> u32 {
        self.vertices.len() as u32
    }

    #[inline(always)]
    pub fn index_count(&self) -> u32 {
        self.indices.len() as u32
    }
}
