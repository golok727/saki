pub mod error;

use std::sync::Arc;

// use crate::gpu::surface::GpuSurface;
// use crate::renderer::Renderer;

pub(crate) use winit::window::Window as WinitWindow;

use crate::app::AppContext;

#[derive(Debug)]
pub struct WindowSpecification {
    pub width: u32,
    pub height: u32,
    pub title: &'static str,
}

pub type WindowId = winit::window::WindowId;

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

pub struct WindowContext<'a> {
    pub app: &'a mut AppContext,
    pub window: &'a mut Window,
}

impl<'a> WindowContext<'a> {
    pub fn new(app: &'a mut AppContext, window: &'a mut Window) -> Self {
        Self { app, window }
    }
}
