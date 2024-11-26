use super::{Quad, TextureId, DEFAULT_UV_COORD};

use crate::math::{Rect, Vec2};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct SceneVertex {
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

impl SceneVertex {
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
    pub vertices: Vec<SceneVertex>,
    pub indices: Vec<u32>,
    // TODO default to white texture instead of optional
    pub texture: Option<TextureId>,
}

impl From<DrawList> for Mesh {
    fn from(mut dl: DrawList) -> Self {
        let vertices: Vec<SceneVertex> = std::mem::take(&mut dl.vertices);
        let indices: Vec<u32> = std::mem::take(&mut dl.indices);

        Self {
            vertices,
            indices,
            texture: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct DrawList {
    pub vertices: Vec<SceneVertex>,
    pub indices: Vec<u32>,
    index_offset: u32,
}

impl DrawList {
    pub fn push_quad(&mut self, quad: &Quad, has_texture: bool) {
        let index_offset = self.index_offset;

        let Rect {
            x,
            y,
            width,
            height,
        } = quad.bounds;

        let vertices = &mut self.vertices;

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
        vertices.push(SceneVertex::new((x, y).into(), color, uvs[0])); // Top-left
        vertices.push(SceneVertex::new((x + width, y).into(), color, uvs[1])); // Top-right
        vertices.push(SceneVertex::new((x, y + height).into(), color, uvs[2])); // Bottom-left
        vertices.push(SceneVertex::new(
            (x + width, y + height).into(),
            color,
            uvs[3],
        )); // Bottom-right

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
