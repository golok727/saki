pub mod async_context;
pub mod events;
pub use async_context::AsyncAppContext;
use skie_draw::paint::SkieAtlas;
use skie_draw::TextSystem;
mod handle;

use crate::window::error::CreateWindowError;
use crate::window::{Window, WindowContext, WindowId, WindowSpecification};
use events::AppEvents;
use handle::AppHandle;
use skie_draw::gpu::GpuContext;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use winit::event::{KeyEvent, MouseScrollDelta, WindowEvent};
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
    // AppContextCallback {
    //     callback: Box<dyn FnOnce(&mut AppContext) + 'static>,
    // },
    // WindowContextCallback {
    //     callback: Box<dyn FnOnce(&mut WindowContext) + 'static>,
    //     window_id: WindowId,
    // },
}

pub(crate) enum Effect {
    UserEvent(AppAction),
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

pub(crate) type AppContextCell = RefCell<AppContext>;
pub(crate) type AppContextRef = Rc<AppContextCell>;

type AppInitCallback = Box<dyn FnOnce(&mut AppContext) + 'static>;
pub type OpenWindowCallback = Box<dyn FnOnce(&mut WindowContext) + 'static>;

pub struct AppContext {
    pub(crate) this: Weak<AppContextCell>,
    pub(crate) jobs: Jobs,
    init_callback: Option<AppInitCallback>,

    pending_updates: usize,
    flushing_effects: bool,
    effects: VecDeque<Effect>,
    pub(crate) app_events: AppEvents,

    pending_user_events: ahash::AHashSet<AppAction>,

    pub(crate) text_system: Arc<TextSystem>,

    pub(crate) texture_system: Arc<SkieAtlas>,

    pub(crate) windows: ahash::AHashMap<WindowId, Window>,

    pub(crate) gpu: Arc<GpuContext>,
}

#[cfg(target_os = "windows")]
static DEFAULT_FONT: &[u8] = include_bytes!("C:\\Windows\\Fonts\\segoeui.ttf");

// #[cfg(target_os = "macos")]
// static DEFAULT_FONT: &[u8] = include_bytes!("/System/Library/Fonts/Supplemental/Arial.ttf");
//
// #[cfg(target_os = "linux")]
// static DEFAULT_FONT: &[u8] = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf");

impl AppContext {
    fn new(handle: &mut AppHandle) -> AppContextRef {
        let jobs = Jobs::new(Some(7));

        let gpu = Arc::new(pollster::block_on(GpuContext::new()).unwrap());

        let texture_system = Arc::new(SkieAtlas::new(gpu.clone()));

        let text_system = TextSystem::default();

        #[cfg(target_os = "windows")]
        {
            let _ = text_system.add_fonts(vec![std::borrow::Cow::Borrowed(DEFAULT_FONT)]);
        }

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
                text_system: Arc::new(text_system),
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

    pub fn text_system(&self) -> &Arc<TextSystem> {
        &self.text_system
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
        log::trace!("Queueing window of specs: {:#?}", &specs);
        self.update(|app| {
            app.push_app_event(AppUpdateEvent::CreateWindow {
                specs,
                callback: Box::new(f),
            });
        });
    }

    fn create_window(
        &mut self,
        specs: &WindowSpecification,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<Window, CreateWindowError> {
        let window = Window::new(self, event_loop, specs)?;

        Ok(window)
    }

    fn handle_window_create_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        specs: WindowSpecification,
        callback: OpenWindowCallback,
    ) {
        log::trace!("Creating window. \n Spec: {:#?}", &specs);
        if let Ok(mut window) = self.create_window(&specs, event_loop) {
            let mut context = WindowContext::new(self, &mut window);

            callback(&mut context);

            let _ = self.windows.insert(window.id(), window);
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
        self.jobs.spawn(f(self.to_async()))
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
                } // AppUpdateEvent::AppContextCallback { callback } => callback(self),
                  // AppUpdateEvent::WindowContextCallback { .. } => {
                  //     let window = self.windows.remove(&window_id);
                  //     if let Some(mut window) = window {
                  //         let mut cx = WindowContext::new(self, &mut window);
                  //         callback(&mut cx);
                  //         self.windows.insert(window.id(), window);
                  //     }
                  // }
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
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(dx, dy),
                ..
            } => {
                let window = self.windows.get_mut(&window_id).expect("expected a window");
                window.handle_scroll_wheel(dx, dy);
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
