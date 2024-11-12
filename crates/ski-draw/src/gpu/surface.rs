use std::cell::Cell;

#[derive(Debug, Clone)]
pub struct SurfaceSpecs {
    pub width: u32,
    pub height: u32,
}

pub struct GpuSurface {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    dirty: Cell<bool>,
}

impl GpuSurface {
    pub(super) fn new(surface: wgpu::Surface<'static>, config: wgpu::SurfaceConfiguration) -> Self {
        Self {
            surface,
            config,
            dirty: Cell::new(false),
        }
    }

    pub fn get_current_texture(&self) -> wgpu::SurfaceTexture {
        self.surface.get_current_texture().unwrap()
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
}
