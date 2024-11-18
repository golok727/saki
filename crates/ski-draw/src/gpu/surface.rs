use super::error::GpuSurfaceCreateError;
use super::GpuContext;

use std::cell::Cell;

#[derive(Debug, Clone)]
pub struct GpuSurfaceSpecification {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct GpuSurface {
    // for now we will remove this
    dirty: Cell<bool>,

    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
}

impl GpuSurface {
    pub(super) fn new(surface: wgpu::Surface<'static>, config: wgpu::SurfaceConfiguration) -> Self {
        Self {
            surface,
            config,
            dirty: Cell::new(false),
        }
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
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
