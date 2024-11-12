// this provides an abstraction over the wgpu api; too lazy to move to another crate
pub mod error;
pub mod surface;

#[derive(Debug)]
pub struct GpuContext {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) instance: wgpu::Instance,
    pub(crate) adapter: wgpu::Adapter,
}

impl GpuContext {
    pub async fn new() -> Result<Self, error::GpuContextCreateError> {
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
            .ok_or(error::GpuContextCreateError::AdapterMissing)?;

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
            .map_err(error::GpuContextCreateError::RequestDeviceError)?;

        Ok(Self {
            device,
            queue,
            instance,
            adapter,
        })
    }

    pub fn create_command_encoder(&self, label: Option<&str>) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label })
    }

    pub fn create_shader(&self) {
        todo!("create_shader is not implemented yet");
    }
    
    pub fn create_texture(&self, descriptor: &wgpu::TextureDescriptor) -> wgpu::Texture {
        self.device.create_texture(descriptor)
    }
}
