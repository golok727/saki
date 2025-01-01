use crate::Vec2;

use super::{Color, DrawVert, TextureId, WHITE_UV};

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<DrawVert>,
    pub indices: Vec<u32>,
    pub texture: TextureId,
}

impl Mesh {
    #[inline(always)]
    pub fn colored_vertex(&mut self, pos: Vec2<f32>, color: Color) {
        self.vertices.push(DrawVert::new(pos, color, WHITE_UV));
    }

    #[inline(always)]
    pub fn add_vertex(&mut self, vertex: DrawVert) {
        self.vertices.push(vertex)
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
    pub fn reserve_primitive(&mut self, vertex_count: usize, index_count: usize) {
        self.vertices.reserve(vertex_count);
        self.indices.reserve(index_count);
    }

    #[inline(always)]
    pub fn reserve_triangles(&mut self, n: usize) {
        self.indices.reserve(3 * n);
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
