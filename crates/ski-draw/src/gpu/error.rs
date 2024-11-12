use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum GpuContextCreateError {
    #[error("wgpu: unable to get adapter")]
    AdapterMissing,
    #[error("wgpu: request device error ({0})")]
    RequestDeviceError(wgpu::RequestDeviceError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub struct GpuSurfaceCreateError(#[from] pub wgpu::CreateSurfaceError);
