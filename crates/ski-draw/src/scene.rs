#[derive(Debug, Clone, Default)]
pub struct Scene {
    quads: Vec<Quad>,
}

impl Scene {
    pub fn add_quad(&mut self, quad: Quad) {
        self.quads.push(quad)
    }

    pub fn quads(&self) -> &[Quad] {
        &self.quads
    }
}

#[derive(Debug, Clone)]
pub struct Quad {
    pub x: f32,
    pub y: f32,
    pub height: u32,
    pub width: u32,
    pub color: wgpu::Color,
}

impl Quad {
    pub fn get_data(&self) {}
}
