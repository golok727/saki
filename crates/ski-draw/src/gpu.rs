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
                })
            ).await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &(wgpu::DeviceDescriptor {
                    label: Some("GPUContext device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits
                        ::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                }),
                None
            ).await
            .unwrap();

        Self {
            device,
            queue,
            instance,
            adapter,
        }
    }

    #[inline(always)]
    pub fn device(&mut self) -> &mut wgpu::Device {
        &mut self.device
    }

    #[inline(always)]
    pub fn queue(&mut self) -> &mut wgpu::Queue {
        &mut self.queue
    }

    #[inline(always)]
    pub fn instance(&mut self) -> &mut wgpu::Instance {
        &mut self.instance
    }

    #[inline(always)]
    pub fn adapter(&mut self) -> &mut wgpu::Adapter {
        &mut self.adapter
    }
}
