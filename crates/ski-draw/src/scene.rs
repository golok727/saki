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
}

#[derive(Debug, Clone)]
pub struct Quad {
    pub bounds: Rect<f32>,
    pub background_color: wgpu::Color,
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
