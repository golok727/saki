use super::error::GpuSurfaceCreateError;
use super::GpuContext;

use crate::renderer::MockRenderTarget;
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

    // todo change this
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

impl MockRenderTarget for GpuSurface {
    fn update(&mut self, gpu: &GpuContext) {
        self.sync(gpu);
    }

    fn get_view(&self) -> wgpu::TextureView {
        self.create_view()
    }

    fn postrender(&mut self) {
        self.present();
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.resize(width, height)
    }

    fn get_texture(&self) -> &wgpu::Texture {
        unimplemented!()
    }
}

impl GpuContext {
    pub fn create_surface(
        &self,
        screen: impl Into<wgpu::SurfaceTarget<'static>>,
        specs: &GpuSurfaceSpecification,
    ) -> Result<GpuSurface, GpuSurfaceCreateError> {
        let width = specs.width.max(1);
        let height = specs.height.max(1);

        let surface = self
            .instance
            .create_surface(screen)
            .map_err(GpuSurfaceCreateError)?;

        let capabilities = surface.get_capabilities(&self.adapter);

        let surface_format = capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(capabilities.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: capabilities.present_modes[0],
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&self.device, &surface_config);

        Ok(GpuSurface::new(surface, surface_config))
    }
}
