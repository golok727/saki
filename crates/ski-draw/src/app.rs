use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::gpu::GpuContext;

use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowAttributes,
};

type InitCallback = Box<dyn FnOnce(&mut App) + 'static>;
type FrameCallback = Box<dyn FnOnce(&mut App) + 'static>;

pub struct App {
    init_callback: Option<InitCallback>,
    frame_callbacks: Rc<RefCell<Vec<FrameCallback>>>,

    window: Option<Arc<winit::window::Window>>,
    // for now
    pub gpu: Arc<GpuContext>,
}

impl App {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let gpu = pollster::block_on(GpuContext::new());

        Self {
            window: None,
            init_callback: None,
            gpu: Arc::new(gpu),
            frame_callbacks: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn gpu(&self) -> &Arc<GpuContext> {
        &self.gpu
    }

    pub fn window_handle(&self) -> &Arc<winit::window::Window> {
        let window = self.window.as_ref().unwrap();
        window
    }

    pub fn on_next_frame<F>(&mut self, f: F)
    where
        F: FnOnce(&mut App) + 'static,
    {
        RefCell::borrow_mut(&self.frame_callbacks).push(Box::new(f))
    }

    pub fn run<F>(&mut self, f: F)
    where
        F: FnOnce(&mut App) + 'static,
    {
        self.init_callback = Some(Box::new(f));
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        if let Err(err) = event_loop.run_app(self) {
            println!("Error running app: Error: {}", err);
        };
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        log::info!("resumed!");
        if self.window.is_none() {
            let attr = WindowAttributes::default()
                .with_inner_size(winit::dpi::PhysicalSize {
                    width: 1280,
                    height: 920,
                })
                .with_title("ski");

            let window = event_loop.create_window(attr).unwrap();

            self.window = Some(Arc::new(window));

            if let Some(cb) = self.init_callback.take() {
                cb(self);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                let callbacks = self.frame_callbacks.clone();
                for callback in callbacks.take() {
                    callback(self);
                }
            }
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => {
                if let Some(window) = &self.window {
                    // TODO add wm and close window only
                    if window.id() == window_id {
                        event_loop.exit();
                        log::info!("Bye!");
                    }
                }
            }
            _ => {
                //
            }
        }
    }
}
