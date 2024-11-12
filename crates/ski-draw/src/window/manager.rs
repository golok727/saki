use super::{error::CreateWindowError, Window, WindowSpecification};
use crate::gpu::GpuContext;

use std::{collections::HashMap, sync::Arc};
use winit::window::WindowId;

#[derive(Debug, Default)]
pub struct WindowManager {
    pub(crate) windows: HashMap<WindowId, Window>,
}

impl WindowManager {
    pub fn create_window(
        &mut self,
        _gpu: Arc<GpuContext>,
        event_loop: &winit::event_loop::ActiveEventLoop,
        specs: &WindowSpecification,
    ) -> Result<(), CreateWindowError> {
        let width = specs.width;
        let height = specs.height;

        // TODO make a attribute builder
        let attr = winit::window::WindowAttributes::default()
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
            .with_title(specs.title);

        // TODO handle error
        let winit_window = event_loop.create_window(attr).map_err(CreateWindowError)?;
        let window_id = winit_window.id();
        let winit_handle = Arc::new(winit_window);

        let window = Window { winit_handle };

        let _ = self.windows.insert(window_id, window).is_some();

        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    pub fn remove(&mut self, id: &WindowId) {
        let _ = self.windows.remove(id);
    }

    pub fn get(&self, id: &WindowId) -> Option<&Window> {
        self.windows.get(id)
    }

    pub fn get_mut(&mut self, id: &WindowId) -> Option<&mut Window> {
        self.windows.get_mut(id)
    }

    pub fn has(&self, id: &WindowId) -> bool {
        self.windows.contains_key(id)
    }
}
