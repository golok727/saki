use crate::GpuContext;
use anyhow::Result;

use super::Canvas;

pub trait CanvasSurface {
    type PaintOutput;
    fn resize(&mut self, gpu: &GpuContext, new_width: u32, new_height: u32);
    fn paint(&mut self, canvas: &mut Canvas) -> Result<Self::PaintOutput>;
}
