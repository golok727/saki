use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use crate::gpu::GpuContext;
use crate::window::error::CreateWindowError;
use crate::window::{Window, WindowId, WindowSpecification};

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

    // we will move this to a task pool;
    windows_to_open: RefCell<Vec<(WindowSpecification, OpenWindowCallback)>>,

    #[allow(dead_code)]
    windows: HashMap<WindowId, Rc<RefCell<Window>>>,
    // pub for now
    pub gpu: Arc<GpuContext>,
}

impl App {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        // TODO handle error
        let gpu = pollster::block_on(GpuContext::new()).unwrap();

        Self {
            init_callback: None,
            windows_to_open: RefCell::new(vec![]),
            windows: HashMap::new(),
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

    fn insert_window(
        &mut self,
        specs: &WindowSpecification,
        event_loop: &winit::event_loop::ActiveEventLoop,
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
    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        log::info!("App Resumed!");

        if let Some(cb) = self.init_callback.take() {
            log::info!("Init callback start");
            cb(self);
        }

        for (spec, callback) in self.windows_to_open.take() {
            // let gpu = Arc::clone(&self.gpu);

            log::info!("Creating window. \n Spec: {:#?}", &spec);
            if let Ok(id) = self.insert_window(&spec, event_loop) {
                log::info!("Window created");

                let window = self.windows.get(&id).cloned();
                let window = window.expect("expected a window");

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
                self.windows.remove(&window_id);

                if self.windows.is_empty() && !event_loop.exiting() {
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
