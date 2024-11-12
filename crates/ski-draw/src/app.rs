use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use crate::gpu::GpuContext;
use crate::window::error::CreateWindowError;
use crate::window::{Window, WindowContext, WindowId, WindowSpecification};

use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

type InitCallback = Box<dyn FnOnce(&mut AppContext) + 'static>;
type FrameCallback = Box<dyn Fn(&mut AppContext) + 'static>;
type OpenWindowCallback = Box<dyn FnOnce(&mut WindowContext) + 'static>;

pub struct App(Rc<RefCell<AppContext>>);

impl App {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(AppContext::new())
    }

    pub fn run<F>(&mut self, f: F)
    where
        F: FnOnce(&mut AppContext) + 'static,
    {
        self.0.borrow_mut().run(f);
    }
}

pub struct AppContext {
    init_callback: Option<InitCallback>,
    frame_callbacks: Rc<RefCell<Vec<FrameCallback>>>,

    // we will move this to a task pool;
    open_window_callbacks: RefCell<Vec<(WindowSpecification, OpenWindowCallback)>>,

    #[allow(dead_code)]
    windows: HashMap<WindowId, Rc<RefCell<Window>>>,
    contains_queued_windows: Cell<bool>,
    // pub for now
    pub gpu: Arc<GpuContext>,
}

impl AppContext {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Rc<RefCell<Self>> {
        // TODO handle error
        let gpu = pollster::block_on(GpuContext::new()).unwrap();

        Rc::new(RefCell::new(Self {
            init_callback: None,
            open_window_callbacks: RefCell::new(vec![]),
            // start windows
            windows: HashMap::new(),
            contains_queued_windows: Cell::new(false),
            // end windows
            gpu: Arc::new(gpu),
            frame_callbacks: Rc::new(RefCell::new(Vec::new())),
        }))
    }

    pub fn gpu(&self) -> &Arc<GpuContext> {
        &self.gpu
    }

    pub fn on_next_frame<F>(&mut self, f: F)
    where
        F: Fn(&mut AppContext) + 'static,
    {
        RefCell::borrow_mut(&self.frame_callbacks).push(Box::new(f))
    }

    pub fn open_window<F>(&mut self, specs: WindowSpecification, f: F)
    where
        F: Fn(&mut WindowContext) + 'static,
    {
        RefCell::borrow_mut(&self.open_window_callbacks).push((specs, Box::new(f)));
        self.contains_queued_windows.set(true);
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
        F: FnOnce(&mut AppContext) + 'static,
    {
        self.init_callback = Some(Box::new(f));
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        if let Err(err) = event_loop.run_app(self) {
            println!("Error running app: Error: {}", err);
        };
    }

    fn run_open_window_callbacks(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.contains_queued_windows.set(false);

        for (spec, callback) in self.open_window_callbacks.take() {
            // let gpu = Arc::clone(&self.gpu);

            log::info!("Creating window. \n Spec: {:#?}", &spec);
            if let Ok(id) = self.insert_window(&spec, event_loop) {
                log::info!("Window created");

                let window = self.windows.get(&id).cloned();
                let window = window.expect("expected a window");

                let mut window_mut = window.borrow_mut();

                let mut context = WindowContext::new(self, &mut window_mut);

                callback(&mut context);
            } else {
                log::error!("Error creating window")
            }
        }
    }
}

impl ApplicationHandler for AppContext {
    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.contains_queued_windows.get() {
            self.run_open_window_callbacks(event_loop);
        }
    }

    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        log::info!("App Resumed!");

        if let Some(cb) = self.init_callback.take() {
            log::info!("Init callback start");
            cb(self);
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
