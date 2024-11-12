use std::cell::{Cell, RefCell};

#[derive(Debug, Clone)]
pub struct GpuSurfaceSpecification {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct GpuSurface {
    // for now we will remove this
    dirty: Cell<bool>,

    pub texture: RefCell<Option<wgpu::SurfaceTexture>>,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
}

impl GpuSurface {
    pub(super) fn new(surface: wgpu::Surface<'static>, config: wgpu::SurfaceConfiguration) -> Self {
        Self {
            surface,
            config,
            texture: RefCell::new(None),
            dirty: Cell::new(false),
        }
    }

    pub fn create_view(&self) -> wgpu::TextureView {
        let mut texture = self.texture.borrow_mut();
        if texture.is_none() {
            *texture = Some(self.surface.get_current_texture().unwrap());
        }

        texture
            .as_ref()
            .unwrap()
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if self.config.width != width || self.config.height != height {
            self.config.width = width;
            self.config.height = height;

            self.dirty.set(true);
        }
    }

    pub fn sync(&self, gpu: &super::GpuContext) {
        if self.dirty.get() {
            self.surface.configure(&gpu.device, &self.config);
            self.dirty.set(false);
        }
    }

    pub fn present(&self) {
        let tex = self.texture.take().unwrap();
        tex.present();
    }
}
