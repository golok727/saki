use super::{Window, WindowSpecification};
use crate::{gpu::GpuContext, Renderer};
use std::{collections::HashMap, sync::Arc};
use winit::window::WindowId;

#[derive(Debug, Default)]
pub struct WindowManager {
    pub(crate) windows: HashMap<WindowId, Window>,
}

impl WindowManager {
    pub fn create_window(
        &self,
        gpu: Arc<GpuContext>,
        event_loop: &winit::event_loop::ActiveEventLoop,
        specs: &WindowSpecification,
    ) {
        let width = specs.width;
        let height = specs.height;

        // TODO make a attribute builder
        let attr = winit::window::WindowAttributes::default()
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
            .with_title(specs.title);

        // TODO handle error
        let winit_window = event_loop.create_window(attr).unwrap();
        let winit_handle = Arc::new(winit_window);

        let window = Window {
            winit_handle,
            renderer: todo!(),
        };

        todo!()
    }

    pub fn get(&self, id: &WindowId) -> bool {
        false
    }
    pub fn has(&self, id: &WindowId) -> bool {
        false
    }
}
