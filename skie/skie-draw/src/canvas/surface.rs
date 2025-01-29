use crate::{gpu, GpuContext};
use anyhow::Result;

use super::Canvas;

pub trait CanvasSurface {
    type PaintOutput;
    fn paint(&mut self, canvas: &mut Canvas) -> Result<Self::PaintOutput>;
    fn configure(&mut self, gpu: &GpuContext, config: &CanvasSurfaceConfig);
    fn get_config(&self) -> CanvasSurfaceConfig;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasSurfaceConfig {
    pub width: u32,
    pub height: u32,
    pub format: gpu::TextureFormat,
    pub usage: gpu::TextureUsages,
}

impl Default for CanvasSurfaceConfig {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            format: gpu::TextureFormat::Rgba8Unorm,
            usage: gpu::TextureUsages::RENDER_ATTACHMENT,
        }
    }
}
