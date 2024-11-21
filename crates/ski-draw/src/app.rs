use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::{Rc, Weak};
use std::sync::Arc;

use crate::gpu::GpuContext;
use crate::jobs::Jobs;
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
        {
            let app = &mut *self.0.borrow_mut();
            app.init_callback = Some(Box::new(f));
        }

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
}

pub struct AppContext {
    pub(crate) this: Weak<RefCell<Self>>,

    pub(self) init_callback: Option<InitCallback>,

    pub(crate) jobs: Jobs,

    pending_user_events: HashSet<UserEvent>,
    pending_updates: usize,
    flushing_effects: bool,
    effects: VecDeque<Effect>,
    app_events: RefCell<Vec<AppUpdateUserEvent>>,
    windows: HashMap<WindowId, Window>,
    // pub for now
    pub(crate) gpu: Arc<GpuContext>,
}

impl AppContext {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Rc<RefCell<Self>> {
        // TODO handle error
        let gpu = pollster::block_on(GpuContext::new()).unwrap();

        // FIXME
        let jobs = Jobs::new(Some(7));

        Rc::new_cyclic(|this| {
            RefCell::new(Self {
                this: this.clone(),
                init_callback: None,

                app_events: Default::default(),
                pending_user_events: HashSet::new(),

                pending_updates: 0,
                flushing_effects: false,
                effects: VecDeque::new(),

                jobs,

                windows: HashMap::new(),
                gpu: Arc::new(gpu),
            })
        })
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

    pub fn change_bg(&mut self, window_id: WindowId, color: (f64, f64, f64)) {
        self.update(|app| {
            app.push_app_event(AppUpdateUserEvent::ChangeWindowBg { window_id, color });
        })
    }

    pub fn set_timeout(&self, f: impl FnOnce(&mut Self) + 'static, timeout: std::time::Duration) {
        let jobs = self.jobs.clone();
        let this = self.this.clone();

        self.jobs
            .spawn_local(async move {
                jobs.timer(timeout).await;
                let app = this.upgrade().expect("app was released");
                let mut app = app.borrow_mut();
                f(&mut app)
            })
            .detach();
    }

    pub fn open_window<F>(&mut self, specs: WindowSpecification, f: F)
    where
        F: Fn(&mut WindowContext) + 'static,
    {
        self.update(|app| app.request_create_window(specs, f));
    }

    #[inline]
    fn request_create_window<F>(&mut self, specs: WindowSpecification, f: F)
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
        let window = Window::new(event_loop, specs, Arc::clone(&self.gpu))?;
        let window_id = window.handle.id();

        Ok((window_id, window))
    }

    fn handle_app_update_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        for event in self.app_events.take() {
            match event {
                window_create_event @ AppUpdateUserEvent::CreateWindow { .. } => {
                    self.handle_window_create_event(event_loop, window_create_event);
                }
                AppUpdateUserEvent::ChangeWindowBg { window_id, color } => {
                    if let Some(window) = self.windows.get_mut(&window_id) {
                        window.set_bg_color(color.0, color.1, color.2);
                    }
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

    fn handle_user_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: UserEvent,
    ) {
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

    fn handle_window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => {
                let width = size.width;
                let height = size.height;
                let window = self.windows.get_mut(&window_id).expect("expected a window");
                window.handle_resize(width, height);
            }
            WindowEvent::RedrawRequested => {
                let window = self.windows.get_mut(&window_id).expect("expected a window");
                window.paint();
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
                // TODO close child windows
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

    fn handle_on_resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        log::info!("Initializing...");

        if let Some(cb) = self.init_callback.take() {
            cb(self);
        }

        log::info!("Initialized!");
    }

    fn handle_about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.jobs.run_foregound_tasks();
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        let mut lock = self.0.borrow_mut(); 
        lock.handle_user_event(event_loop, event);
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let mut lock = self.0.borrow_mut(); 
        lock.handle_about_to_wait(event_loop); 
    }

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let mut lock = self.0.borrow_mut(); 
        lock.handle_on_resumed(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,

    ) {
        let mut lock = self.0.borrow_mut(); 
        lock.handle_window_event(event_loop, window_id, event)
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
    ChangeWindowBg {
        window_id: WindowId,
        color: (f64, f64, f64),
    },
    #[allow(unused)]
    Other,
}

pub(crate) enum Effect {
    UserEvent(UserEvent),

    #[allow(unused)]
    Other,
}
