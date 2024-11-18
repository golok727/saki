use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
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

pub(crate) static EVENT_LOOP_PROXY: Mutex<Option<winit::event_loop::EventLoopProxy<UserEvent>>> =
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserEvent {
    AppUpdate,
    Quit,
}

pub(crate) enum AppUpdateUserEvent {
    CreateWindow {
        specs: WindowSpecification,
        callback: OpenWindowCallback,
    },
    #[allow(unused)]
    Other,
}

pub(crate) enum Effect {
    UserEvent(UserEvent),

    #[allow(unused)]
    Other,
}

pub struct AppContext {
    init_callback: Option<InitCallback>,
    frame_callbacks: Rc<RefCell<Vec<FrameCallback>>>,

    pending_user_events: HashSet<UserEvent>,
    pending_updates: usize,
    flushing_effects: bool,
    effects: VecDeque<Effect>,
    app_events: RefCell<Vec<AppUpdateUserEvent>>,
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

            app_events: Default::default(),
            pending_user_events: HashSet::new(),

            pending_updates: 0,
            flushing_effects: false,
            effects: VecDeque::new(),

            frame_callbacks: Rc::new(RefCell::new(Vec::new())),

            windows: HashMap::new(),
            gpu: Arc::new(gpu),
        }))
    }

    pub fn gpu(&self) -> &Arc<GpuContext> {
        &self.gpu
    }

    pub fn update<R>(&mut self, cb: impl FnOnce(&mut Self) -> R) -> R {
        self.pending_updates += 1;
        let res = cb(self);

        if !self.flushing_effects && self.pending_updates == 1 {
            self.flush_effects();
        }
        self.pending_updates -= 1;

        res
    }

    pub(crate) fn push_effect(&mut self, effect: Effect) {
        match effect {
            Effect::UserEvent(event) => {
                if !self.pending_user_events.insert(event) {
                    return;
                }
            }
            _ => unimplemented!(),
        }

        self.effects.push_back(effect);
    }

    fn flush_effects(&mut self) {
        self.flushing_effects = true;

        while let Some(effect) = self.effects.pop_front() {
            match effect {
                Effect::UserEvent(event) => self.notify_user_event(event),
                _ => unimplemented!(),
            }
        }

        self.flushing_effects = false;
    }

    fn notify_user_event(&self, event: UserEvent) {
        log::debug!("Notify");
        AppContext::use_proxy(|proxy| {
            if let Err(error) = proxy.send_event(event) {
                log::error!("Error sending: {}", &error)
            }
        });
    }

    pub(crate) fn push_app_event(&mut self, event: AppUpdateUserEvent) {
        RefCell::borrow_mut(&self.app_events).push(event);
        self.push_effect(Effect::UserEvent(UserEvent::AppUpdate))
    }

    pub fn quit(&mut self) {
        self.update(|app| {
            app.push_effect(Effect::UserEvent(UserEvent::Quit));
        })
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
        log::debug!("Queueing window");
        self.update(|app| app.request_create_window(specs, f));
    }

    #[inline]
    pub(crate) fn request_create_window<F>(&mut self, specs: WindowSpecification, f: F)
    where
        F: Fn(&mut WindowContext) + 'static,
    {
        self.push_app_event(AppUpdateUserEvent::CreateWindow {
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
        let event_loop: EventLoop<UserEvent> = EventLoop::with_user_event()
            .build()
            .expect("error creating event_loop.");

        let proxy = event_loop.create_proxy();

        *EVENT_LOOP_PROXY.lock() = Some(proxy);

        if let Err(err) = event_loop.run_app(self) {
            println!("Error running app: Error: {}", err);
        } else {
            EVENT_LOOP_PROXY.lock().take();
        };
    }

    fn handle_app_update_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        for event in self.app_events.take() {
            match event {
                window_create_event @ AppUpdateUserEvent::CreateWindow { .. } => {
                    self.handle_window_create_event(event_loop, window_create_event);
                }
                _ => todo!(),
            }
        }
    }

    // please pass in the AppEvent::CreateWindow or it will panic
    fn handle_window_create_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_create_event: AppUpdateUserEvent,
    ) {
        match window_create_event {
            AppUpdateUserEvent::CreateWindow { specs, callback } => {
                log::info!("Creating window. \n Spec: {:#?}", &specs);
                if let Ok((id, mut window)) = self.create_window(&specs, event_loop) {
                    let mut context = WindowContext::new(self, &mut window);

                    callback(&mut context);

                    let _ = self.windows.insert(id, window);
                } else {
                    log::error!("Error creating window")
                }
            }
            _ => unreachable!(),
        }
    }

    pub(crate) fn use_proxy<F>(f: F)
    where
        F: FnOnce(&winit::event_loop::EventLoopProxy<UserEvent>),
    {
        if let Some(proxy) = EVENT_LOOP_PROXY.lock().as_ref() {
            f(proxy)
        }
    }
}

impl ApplicationHandler<UserEvent> for AppContext {
    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        log::debug!("Handling user event: {:#?}", &event);

        self.pending_user_events.remove(&event);

        match event {
            UserEvent::AppUpdate => self.handle_app_update_event(event_loop),
            UserEvent::Quit => {
                event_loop.exit();
                EVENT_LOOP_PROXY.lock().take();
                log::info!("Bye!");
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
