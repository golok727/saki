use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::gpu::GpuContext;
use crate::window::{Window, WindowManager, WindowSpecification};

use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

pub struct WindowContext<'a> {
    pub app: &'a mut App,
    pub window: &'a mut Window,
}

type InitCallback = Box<dyn FnOnce(&mut App) + 'static>;
type FrameCallback = Box<dyn Fn(&mut App) + 'static>;
type OpenWindowCallback = Box<dyn FnOnce(&mut WindowContext) + 'static>;

pub struct App {
    init_callback: Option<InitCallback>,
    frame_callbacks: Rc<RefCell<Vec<FrameCallback>>>,

    windows_to_open: RefCell<Vec<(WindowSpecification, OpenWindowCallback)>>,

    wm: WindowManager,
    // pub for now
    pub gpu: Arc<GpuContext>,
}

impl App {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        // TODO handle error
        let gpu = pollster::block_on(GpuContext::new()).unwrap();
        let window_manager = WindowManager::new();

        Self {
            init_callback: None,
            windows_to_open: RefCell::new(vec![]),
            wm: window_manager,
            gpu: Arc::new(gpu),
            frame_callbacks: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn gpu(&self) -> &Arc<GpuContext> {
        &self.gpu
    }

    pub fn on_next_frame<F>(&mut self, f: F)
    where
        F: Fn(&mut App) + 'static,
    {
        RefCell::borrow_mut(&self.frame_callbacks).push(Box::new(f))
    }

    pub fn open_window<F>(&mut self, specs: WindowSpecification, f: F)
    where
        F: Fn(&mut WindowContext) + 'static,
    {
        RefCell::borrow_mut(&self.windows_to_open).push((specs, Box::new(f)));
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
        log::info!("App Resumed!");

        if let Some(cb) = self.init_callback.take() {
            log::info!("Init callback start");
            cb(self);
        }

        for (spec, callback) in self.windows_to_open.take() {
            let gpu = Arc::clone(&self.gpu);

            log::info!("Creating window. \n Spec: {:#?}", &spec);
            if let Ok(id) = self.wm.create_window(gpu, event_loop, &spec) {
                log::info!("Window created");
                let window = self.wm.get(&id).expect("window not found");
                let mut window_mut = window.borrow_mut();

                let mut context = WindowContext {
                    app: self,
                    window: &mut window_mut,
                };

                callback(&mut context);
            } else {
                log::error!("Error creating window")
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
                self.wm.remove(&window_id);

                if self.wm.is_empty() && !event_loop.exiting() {
                    // TODO make this better
                    event_loop.exit();
                }
            }
            _ => {
                //
            }
        }
    }
}
