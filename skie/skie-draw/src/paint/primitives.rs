use crate::math::Rect;

#[derive(Debug, Clone)]
pub enum PrimitiveKind {
    Quad(Quad),
}

#[derive(Debug, Clone)]
pub struct Quad {
    pub bounds: Rect<f32>,
    pub background_color: wgpu::Color,
}

impl Quad {
    pub fn with_bgcolor(mut self, r: f64, g: f64, b: f64, a: f64) -> Self {
        self.background_color = wgpu::Color { r, g, b, a };
        self
    }
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.bounds.width = width;
        self.bounds.height = height;
        self
    }

    pub fn with_pos(mut self, x: f32, y: f32) -> Self {
        self.bounds.x = x;
        self.bounds.y = y;
        self
    }
}

impl Default for Quad {
    fn default() -> Self {
        Self {
            bounds: Rect {
                x: 0.,
                y: 0.,
                width: 10.,
                height: 10.,
            },
            background_color: wgpu::Color::WHITE,
        }
    }
}

pub fn quad() -> Quad {
    Quad::default()
}

macro_rules! impl_into_primitive {
    ($t: ty, $kind: tt) => {
        impl From<$t> for PrimitiveKind {
            fn from(val: $t) -> Self {
                PrimitiveKind::$kind(val)
            }
        }
    };
}

impl_into_primitive!(Quad, Quad);
