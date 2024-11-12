pub mod manager;

use std::sync::Arc;

use crate::gpu::surface::GpuSurface;
use crate::renderer::Renderer;
pub use manager::WindowManager;

pub(crate) use winit::window::Window as WinitWindow;

#[derive(Debug, Default)]
pub struct WindowSpecification {
    pub width: u32,
    pub height: u32,
    pub title: &'static str,
}

#[derive(Debug)]
pub struct Window {
    pub(crate) winit_handle: Arc<WinitWindow>,
    pub(crate) renderer: Renderer,
    // pub(crate) surface: GpuSurface,
}

impl Window {
    #[inline]
    pub fn id(&self) -> winit::window::WindowId {
        self.winit_handle.id()
    }
}
