use crate::math::Rect;

#[derive(Debug, Clone)]
pub enum Primitive {
    Quad(Quad),
}

#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub(crate) prims: Vec<Primitive>,
}

impl Scene {
    pub fn push_layer(&mut self) {
        todo!()
    }

    pub fn pop_layer(&mut self) {
        todo!()
    }

    pub fn add(&mut self, prim: impl Into<Primitive>) {
        self.prims.push(prim.into())
    }

    pub fn clear(&mut self) -> Vec<Primitive> {
        let old: Vec<Primitive> = std::mem::take(&mut self.prims);
        old
    }
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
                width: 1.,
                height: 1.,
            },
            background_color: wgpu::Color::RED,
        }
    }
}

pub fn quad() -> Quad {
    Quad::default()
}

macro_rules! impl_into_primitive {
    ($t: ty, $kind: tt) => {
        impl From<$t> for Primitive {
            fn from(val: $t) -> Self {
                Primitive::$kind(val)
            }
        }
    };
}

impl_into_primitive!(Quad, Quad);
