use super::{error::CreateWindowError, Window, WindowSpecification};
use crate::gpu::GpuContext;

use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};
use winit::window::WindowId;

#[derive(Debug, Default)]
pub struct WindowManager {
    pub(crate) windows: HashMap<WindowId, Rc<RefCell<Window>>>,
}

impl WindowManager {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_window(
        &mut self,
        _gpu: Arc<GpuContext>,
        event_loop: &winit::event_loop::ActiveEventLoop,
        specs: &WindowSpecification,
    ) -> Result<WindowId, CreateWindowError> {
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

        let _ = self
            .windows
            .insert(window_id, Rc::new(RefCell::new(window)))
            .is_some();

        Ok(window_id)
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    pub fn remove(&mut self, id: &WindowId) {
        let _ = self.windows.remove(id);
    }

    pub fn get(&self, id: &WindowId) -> Option<Rc<RefCell<Window>>> {
        self.windows.get(id).cloned()
    }

    pub fn has(&self, id: &WindowId) -> bool {
        self.windows.contains_key(id)
    }
}
