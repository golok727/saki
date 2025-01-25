use super::error::GpuSurfaceCreateError;
use super::GpuContext;

#[derive(Debug, Clone)]
pub struct GpuSurfaceSpecification {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct GpuSurface<'a> {
    pub surface: wgpu::Surface<'a>,
    pub config: wgpu::SurfaceConfiguration,
}

impl<'a> GpuSurface<'a> {
    pub(super) fn new(surface: wgpu::Surface<'a>, config: wgpu::SurfaceConfiguration) -> Self {
        Self { surface, config }
    }

    pub fn resize(&mut self, gpu: &GpuContext, new_width: u32, new_height: u32) {
        if self.config.width != new_width || self.config.height != new_height {
            self.config.width = new_width.max(1);
            self.config.height = new_height.max(1);

            self.surface.configure(&gpu.device, &self.config);
            log::trace!(
                "Surface target resize:  width = {} height = {}",
                self.config.width,
                self.config.height
            );
        }
    }
}

impl GpuContext {
    pub fn create_surface<'a, 'surface>(
        &'a self,
        screen: impl Into<wgpu::SurfaceTarget<'surface>>,
        specs: &GpuSurfaceSpecification,
    ) -> Result<GpuSurface<'surface>, GpuSurfaceCreateError> {
        let width = specs.width.max(1);
        let height = specs.height.max(1);

        let surface = self
            .instance
            .create_surface(screen)
            .map_err(GpuSurfaceCreateError)?;

        let capabilities = surface.get_capabilities(&self.adapter);

        // let surface_format = capabilities
        //     .formats
        //     .iter()
        //     .find(|f| f.is_srgb())
        //     .copied()
        //     .unwrap_or(capabilities.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
            // TODO: make format configurable
            format: wgpu::TextureFormat::Rgba8Unorm,
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
