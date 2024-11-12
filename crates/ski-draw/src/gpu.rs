// this provides an abstraction over the wgpu api; too lazy to move to another crate
pub mod surface;

#[derive(Debug)]
pub struct GpuContext {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) instance: wgpu::Instance,
    pub(crate) adapter: wgpu::Adapter,
}

impl GpuContext {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(
                &(wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    force_fallback_adapter: false,
                    compatible_surface: None,
                }),
            )
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &(wgpu::DeviceDescriptor {
                    label: Some("GPUContext device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                }),
                None,
            )
            .await
            .unwrap();

        Self {
            device,
            queue,
            instance,
            adapter,
        }
    }

    pub fn create_surface(
        &self,
        screen: impl Into<wgpu::SurfaceTarget<'static>>,
        specs: &surface::GpuSurfaceSpecification,
    ) -> surface::GpuSurface {
        let width = specs.width.max(1);
        let height = specs.height.max(1);

        let surface = self.instance.create_surface(screen).unwrap();

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

        surface::GpuSurface::new(surface, surface_config)
    }
}
