use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use crate::gpu::GpuContext;
use crate::window::error::CreateWindowError;
use crate::window::{Window, WindowContext, WindowId, WindowSpecification};

use winit::event_loop::EventLoop;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use parking_lot::Mutex;

pub(crate) static EVENT_LOOP_PROXY: Mutex<Option<winit::event_loop::EventLoopProxy<AppAction>>> =
    Mutex::new(None);

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    AppUpdate,
    Quit,
}

pub(crate) enum AppEvent {
    CreateWindow {
        specs: WindowSpecification,
        callback: OpenWindowCallback,
    },
}

pub struct AppContext {
    init_callback: Option<InitCallback>,
    frame_callbacks: Rc<RefCell<Vec<FrameCallback>>>,

    app_events: RefCell<Vec<AppEvent>>,
    windows: HashMap<WindowId, Window>,
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
            windows: HashMap::new(),
            app_events: Default::default(),
            gpu: Arc::new(gpu),
            frame_callbacks: Rc::new(RefCell::new(Vec::new())),
        }))
    }

    pub(crate) fn push_event(&self, event: AppEvent) {
        RefCell::borrow_mut(&self.app_events).push(event);

        AppContext::use_proxy(|proxy| {
            if let Err(error) = proxy.send_event(AppAction::AppUpdate) {
                log::error!("Error sending AppUpdateEvent: {}", &error)
            }
        });
    }

    pub fn quit(&self) {
        AppContext::use_proxy(|proxy| {
            if let Err(error) = proxy.send_event(AppAction::Quit) {
                log::error!("Error sending QuitEvent: {}", &error)
            }
        })
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
        self.push_event(AppEvent::CreateWindow {
            specs,
            callback: Box::new(f),
        })
    }

    fn create_window(
        &mut self,
        specs: &WindowSpecification,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<(WindowId, Window), CreateWindowError> {
        let width = specs.width;
        let height = specs.height;

        // TODO make a spec builder
        let attr = winit::window::WindowAttributes::default()
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
            .with_title(specs.title);

        let winit_window = event_loop.create_window(attr).map_err(CreateWindowError)?;
        let window_id = winit_window.id();
        let winit_handle = Arc::new(winit_window);

        let window = Window { winit_handle };

        Ok((window_id, window))
    }

    pub fn run<F>(&mut self, f: F)
    where
        F: FnOnce(&mut AppContext) + 'static,
    {
        self.init_callback = Some(Box::new(f));
        let event_loop: EventLoop<AppAction> = EventLoop::with_user_event()
            .build()
            .expect("error creating event_loop.");

        let proxy = event_loop.create_proxy();

        *EVENT_LOOP_PROXY.lock() = Some(proxy);

        if let Err(err) = event_loop.run_app(self) {
            println!("Error running app: Error: {}", err);
        } else {
            *EVENT_LOOP_PROXY.lock() = None;
        };
    }

    fn handle_app_update_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        for event in self.app_events.take() {
            match event {
                AppEvent::CreateWindow { specs, callback } => {
                    self.handle_window_create_event(event_loop, specs, callback);
                }
            }
        }
    }

    fn handle_window_create_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        specs: WindowSpecification,
        callback: OpenWindowCallback,
    ) {
        log::info!("Creating window. \n Spec: {:#?}", &specs);
        if let Ok((id, mut window)) = self.create_window(&specs, event_loop) {
            log::info!("Window created");

            let mut context = WindowContext::new(self, &mut window);

            log::info!("Calling window init callback");
            callback(&mut context);

            let _ = self.windows.insert(id, window);
        } else {
            log::error!("Error creating window")
        }
    }

    pub(crate) fn use_proxy<F>(f: F)
    where
        F: FnOnce(&winit::event_loop::EventLoopProxy<AppAction>) + 'static,
    {
        if let Some(proxy) = EVENT_LOOP_PROXY.lock().as_ref() {
            f(proxy)
        }
    }
}

impl ApplicationHandler<AppAction> for AppContext {
    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: AppAction) {
        match event {
            AppAction::AppUpdate => self.handle_app_update_event(event_loop),
            AppAction::Quit => {
                event_loop.exit();
                // or winit will cause issues
                *EVENT_LOOP_PROXY.lock() = None;
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        log::info!("App Resumed!");

        if let Some(cb) = self.init_callback.take() {
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
                    self.quit();
                }
            }
            _ => {
                //
            }
        }
    }
}
