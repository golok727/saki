use std::ops::Deref;

use crate::canvas::surface::CanvasSurface;
use crate::{Canvas, GpuContext};
use anyhow::Result;
use wgpu::SurfaceTexture;

use super::surface::CanvasSurfaceConfig;

#[derive(Debug, Clone)]
pub struct GpuSurfaceSpecification {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct BackendRenderTarget<'a> {
    pub surface: wgpu::Surface<'a>,
    pub config: wgpu::SurfaceConfiguration,
}

impl<'a> Deref for BackendRenderTarget<'a> {
    type Target = wgpu::Surface<'a>;

    fn deref(&self) -> &Self::Target {
        &self.surface
    }
}

impl<'a> BackendRenderTarget<'a> {
    pub(super) fn new(surface: wgpu::Surface<'a>, config: wgpu::SurfaceConfiguration) -> Self {
        Self { surface, config }
    }
}

#[derive(Debug)]
pub struct PaintedSurface(SurfaceTexture);

impl PaintedSurface {
    pub fn present(self) {
        self.0.present()
    }
}

impl<'a> CanvasSurface for BackendRenderTarget<'a> {
    type PaintOutput = PaintedSurface;

    fn paint(&mut self, canvas: &mut Canvas) -> Result<Self::PaintOutput> {
        let surface_texture = self.surface.get_current_texture()?;

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        canvas.render_to_texture(&view);

        Ok(PaintedSurface(surface_texture))
    }

    fn configure(&mut self, gpu: &GpuContext, config: &CanvasSurfaceConfig) {
        self.config.width = config.width;
        self.config.height = config.height;
        self.config.usage = config.usage | wgpu::TextureUsages::RENDER_ATTACHMENT;
        self.config.format = config.format;

        self.surface.configure(&gpu.device, &self.config);
    }

    fn get_config(&self) -> CanvasSurfaceConfig {
        CanvasSurfaceConfig {
            width: self.config.width,
            height: self.config.height,
            format: self.config.format,
            usage: self.config.usage,
        }
    }
}

impl Canvas {
    pub fn create_backend_target<'window>(
        &self,
        into_surface_target: impl Into<wgpu::SurfaceTarget<'window>>,
    ) -> Result<BackendRenderTarget<'window>> {
        let gpu = self.renderer.gpu();

        let surface = gpu.instance.create_surface(into_surface_target)?;

        let capabilities = surface.get_capabilities(&gpu.adapter);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | self.surface_config.usage,
            format: self.surface_config.format,
            width: self.surface_config.width,
            height: self.surface_config.height,
            present_mode: capabilities.present_modes[0],
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&gpu.device, &surface_config);

        Ok(BackendRenderTarget::new(surface, surface_config))
    }
}
