pub mod error;
pub mod manager;

use std::sync::Arc;

// use crate::gpu::surface::GpuSurface;
// use crate::renderer::Renderer;
pub use manager::WindowManager;

pub(crate) use winit::window::Window as WinitWindow;

#[derive(Debug)]
pub struct WindowSpecification {
    pub width: u32,
    pub height: u32,
    pub title: &'static str,
}

impl Default for WindowSpecification {
    fn default() -> Self {
        Self {
            width: 800,
            height: 800,
            title: "Ski",
        }
    }
}

#[derive(Debug)]
pub struct Window {
    pub(crate) winit_handle: Arc<WinitWindow>,
    // pub(crate) renderer: Renderer,
    // pub(crate) surface: GpuSurface,
}

impl Window {
    #[inline]
    pub fn id(&self) -> winit::window::WindowId {
        self.winit_handle.id()
    }

    pub fn winit_handle(&self) -> &Arc<WinitWindow> {
        &self.winit_handle
    }
}
