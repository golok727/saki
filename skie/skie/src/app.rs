pub mod events;

use crate::window::error::CreateWindowError;
use crate::window::{Window, WindowContext, WindowId, WindowSpecification};
use events::AppEvents;
use skie_draw::gpu::GpuContext;
use skie_draw::paint::atlas::AtlasManager;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::jobs::{Job, Jobs};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppAction {
    AppUpdate,
    Quit,
}

pub(crate) enum AppUpdateEvent {
    CreateWindow {
        specs: WindowSpecification,
        callback: OpenWindowCallback,
    },

    // should we move these two to AppAction ?
    AppContextCallback {
        callback: Box<dyn FnOnce(&mut AppContext) + 'static>,
    },
    WindowContextCallback {
        callback: Box<dyn FnOnce(&mut WindowContext) + 'static>,
        window_id: WindowId,
    },
}

pub(crate) enum Effect {
    UserEvent(AppAction),
}

type ResumedCallback = Box<dyn Fn(&ActiveEventLoop)>;
type UserEventCallback = Box<dyn Fn(&ActiveEventLoop, AppAction)>;
type AboutToWaitCallback = Box<dyn Fn(&ActiveEventLoop)>;
type WindowEventCallback = Box<
    dyn Fn(&winit::event_loop::ActiveEventLoop, winit::window::WindowId, winit::event::WindowEvent),
>;

#[derive(Default)]
pub struct AppHandleCallbacks {
    resumed: Option<ResumedCallback>,
    window_event: Option<WindowEventCallback>,
    about_to_wait: Option<AboutToWaitCallback>,
    user_event: Option<UserEventCallback>,
}

#[derive(Default)]
struct AppHandle {
    callbacks: AppHandleCallbacks,
}

impl AppHandle {
    fn on_resumed(&mut self, callback: ResumedCallback) {
        self.callbacks.resumed = Some(callback);
    }
    fn on_window_event(&mut self, callback: WindowEventCallback) {
        self.callbacks.window_event = Some(callback);
    }

    fn on_user_event(&mut self, callback: UserEventCallback) {
        self.callbacks.user_event = Some(callback);
    }

    fn on_about_to_wait(&mut self, callback: AboutToWaitCallback) {
        self.callbacks.about_to_wait = Some(callback);
    }
}

impl ApplicationHandler<AppAction> for AppHandle {
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(callback) = &self.callbacks.about_to_wait {
            callback(event_loop)
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppAction) {
        if let Some(callback) = &self.callbacks.user_event {
            callback(event_loop, event)
        }
    }

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(callback) = &self.callbacks.resumed {
            callback(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(callback) = &self.callbacks.window_event {
            callback(event_loop, window_id, event)
        }
    }
}

pub struct App {
    cx: AppContextRef,
    handle: AppHandle,
}

impl App {
    pub fn new() -> Self {
        let mut handle = AppHandle::default();
        let cx = AppContext::new(&mut handle);
        Self { cx, handle }
    }

    pub fn run(mut self, on_init: impl FnOnce(&mut AppContext) + 'static) {
        let event_loop: winit::event_loop::EventLoop<AppAction> =
            winit::event_loop::EventLoop::with_user_event()
                .build()
                .expect("error creating event_loop.");

        let proxy = event_loop.create_proxy();

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

        {
            let mut cx = self.cx.borrow_mut();
            cx.init_callback = Some(Box::new(on_init));
            cx.app_events.init(proxy);
        }

        event_loop
            .run_app(&mut self.handle)
            .expect("Error running EventLoop");
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

type AppContextCell = RefCell<AppContext>;
type AppContextRef = Rc<AppContextCell>;

type AppInitCallback = Box<dyn FnOnce(&mut AppContext) + 'static>;
pub type OpenWindowCallback = Box<dyn FnOnce(&mut WindowContext) + 'static>;

#[derive(Clone)]
pub struct AsyncAppContext {
    pub(crate) app: Weak<AppContextCell>,
    pub(crate) jobs: Jobs,
}

impl AsyncAppContext {
    pub(crate) fn handle_on_resumed(&self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let cx = self.app.upgrade().expect("app  released");
        let mut lock = cx.borrow_mut();
        lock.handle_on_resumed(event_loop);
    }

    fn handle_window_event(
        &self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let cx = self.app.upgrade().expect("app released");
        let mut lock = cx.borrow_mut();
        lock.handle_window_event(event_loop, window_id, event);
    }

    fn handle_on_about_to_wait(&self, event_loop: &ActiveEventLoop) {
        // If we put this inside the app it will cause a double borrow
        self.jobs.run_foregound_tasks();

        let cx = self.app.upgrade().expect("app released");
        let mut lock = cx.borrow_mut();
        lock.handle_on_about_to_wait(event_loop);
    }

    fn handle_on_user_event(&self, event_loop: &ActiveEventLoop, event: AppAction) {
        let cx = self.app.upgrade().expect("app released");
        let mut lock = cx.borrow_mut();
        lock.handle_on_user_event(event_loop, event);
    }
}

pub struct AppContext {
    this: Weak<AppContextCell>,
    jobs: Jobs,
    init_callback: Option<AppInitCallback>,

    pending_updates: usize,
    flushing_effects: bool,
    effects: VecDeque<Effect>,
    app_events: AppEvents,

    pending_user_events: ahash::AHashSet<AppAction>,

    pub(crate) texture_system: AtlasManager,

    windows: ahash::AHashMap<WindowId, Window>,

    pub(crate) gpu: Arc<GpuContext>,
}

impl AppContext {
    fn new(handle: &mut AppHandle) -> AppContextRef {
        let jobs = Jobs::new(Some(7));

        let gpu = Arc::new(pollster::block_on(GpuContext::new()).unwrap());

        let texture_system = AtlasManager::new(gpu.clone());

        let cx = Rc::new_cyclic(|this| {
            RefCell::new(Self {
                this: this.clone(),
                jobs,
                init_callback: None,
                gpu,

                pending_updates: 0,
                flushing_effects: false,
                effects: Default::default(),
                app_events: Default::default(),
                pending_user_events: Default::default(),

                texture_system,
                windows: ahash::AHashMap::new(),
            })
        });

        {
            let lock = cx.borrow();

            handle.on_about_to_wait({
                let cx = lock.to_async();

                Box::new(move |event_loop| {
                    cx.handle_on_about_to_wait(event_loop);
                })
            });

            handle.on_window_event({
                let cx = lock.to_async();

                Box::new(move |event_loop, window_id, event| {
                    cx.handle_window_event(event_loop, window_id, event);
                })
            });

            handle.on_resumed({
                let cx = lock.to_async();

                Box::new(move |event_loop| {
                    cx.handle_on_resumed(event_loop);
                })
            });

            handle.on_user_event({
                let cx = lock.to_async();

                Box::new(move |event_loop, user_event| {
                    cx.handle_on_user_event(event_loop, user_event);
                })
            });
        }

        cx
    }

    pub fn to_async(&self) -> AsyncAppContext {
        AsyncAppContext {
            app: self.this.clone(),
            jobs: self.jobs.clone(),
        }
    }

    fn flush_effects(&mut self) {
        self.flushing_effects = true;

        while let Some(effect) = self.effects.pop_front() {
            match effect {
                Effect::UserEvent(event) => self.app_events.notify(event),
            }
        }

        self.flushing_effects = false;
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
        }

        self.effects.push_back(effect);
    }

    pub(crate) fn push_app_event(&mut self, event: AppUpdateEvent) {
        self.app_events.push_event(event);
        self.push_effect(Effect::UserEvent(AppAction::AppUpdate))
    }

    pub fn open_window<F>(&mut self, specs: WindowSpecification, f: F)
    where
        F: Fn(&mut WindowContext) + 'static,
    {
        self.update(|app| {
            app.push_app_event(AppUpdateEvent::CreateWindow {
                specs,
                callback: Box::new(f),
            });
        });

        log::info!("opening a new window");
    }

    fn create_window(
        &mut self,
        specs: &WindowSpecification,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<(WindowId, Window), CreateWindowError> {
        let window = Window::new(
            event_loop,
            Arc::clone(&self.gpu),
            self.texture_system.clone(),
            specs,
        )?;
        let window_id = window.handle.id();

        Ok((window_id, window))
    }

    fn handle_window_create_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        specs: WindowSpecification,
        callback: OpenWindowCallback,
    ) {
        log::trace!("Creating window. \n Spec: {:#?}", &specs);
        if let Ok((id, mut window)) = self.create_window(&specs, event_loop) {
            let mut context = WindowContext::new(self, &mut window);

            callback(&mut context);

            let _ = self.windows.insert(id, window);
        } else {
            log::error!("Error creating window")
        }
    }

    pub fn quit(&mut self) {
        self.update(|app| {
            app.push_effect(Effect::UserEvent(AppAction::Quit));
        })
    }

    pub fn spawn<Fut, R>(&self, f: impl FnOnce(AsyncAppContext) -> Fut) -> Job<R>
    where
        Fut: Future<Output = R> + 'static,
        R: 'static,
    {
        self.jobs.spawn_local(f(self.to_async()))
    }

    pub fn set_timeout(
        &mut self,
        f: impl FnOnce(&mut AppContext) + 'static,
        timeout: std::time::Duration,
    ) {
        self.spawn(|cx| async move {
            cx.jobs.timer(timeout).await;
            let cx = cx.app.upgrade().expect("app released");
            let mut lock = cx.borrow_mut();
            f(&mut lock);
        })
        .detach();
    }

    fn handle_app_update_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        for event in self.app_events.drain() {
            match event {
                AppUpdateEvent::CreateWindow { specs, callback } => {
                    self.handle_window_create_event(event_loop, specs, callback);
                }
                AppUpdateEvent::AppContextCallback { callback } => callback(self),
                AppUpdateEvent::WindowContextCallback {
                    window_id,
                    callback,
                } => {
                    // FIXME:
                    log::info!("WindowContextCallback");
                    // let window = self.windows.remove(&window_id);
                    // if let Some(mut window) = window {
                    //     let mut cx = WindowContext::new(self, &mut window);
                    //     callback(&mut cx);
                    //     self.windows.insert(window.id(), window);
                    // }
                }
            }
        }
    }

    fn handle_on_about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {}

    fn handle_on_user_event(&mut self, event_loop: &ActiveEventLoop, event: AppAction) {
        self.pending_user_events.remove(&event);

        match event {
            AppAction::AppUpdate => self.handle_app_update_event(event_loop),
            AppAction::Quit => {
                event_loop.exit();
                self.app_events.dispose();
                log::info!("Bye!");
            }
        }
    }

    fn handle_on_resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        log::info!("Initializing App...");
        if let Some(cb) = self.init_callback.take() {
            cb(self);
        }
        log::info!("App Initialized!");
    }

    fn handle_window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
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
                self.windows.remove(&window_id);
                if self.windows.is_empty() {
                    self.quit();
                }
            }
            _ => {
                //
            }
        }
    }
}
