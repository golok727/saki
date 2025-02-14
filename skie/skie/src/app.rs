pub mod async_context;
pub mod context;
pub mod events;
pub(crate) mod world;
pub use async_context::AsyncAppContext;
use world::{Entity, World};

use winit::application::ApplicationHandler;

use crate::window::{Window, WindowId, WindowSpecification};
use anyhow::Result;
use events::AppEvents;
use skie_draw::gpu::GpuContext;
use skie_draw::paint::SkieAtlas;
use skie_draw::TextSystem;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::jobs::{Job, Jobs};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserEvent {
    AppUpdate,
    Quit,
}

pub(crate) enum AppUpdateEvent {
    CreateWindow {
        specs: WindowSpecification,
        callback: OpenWindowCallback,
    },
}

pub(crate) enum Effect {
    AppEvent(UserEvent),
}

pub struct App {
    app: AppContextRef,
    jobs: Jobs,
}

impl App {
    pub fn new() -> Self {
        let app = AppContext::new();
        let jobs = app.borrow().jobs.clone();
        Self { app, jobs }
    }

    pub fn run(mut self, on_init: impl FnOnce(&mut AppContext) + 'static) {
        let event_loop: winit::event_loop::EventLoop<UserEvent> =
            winit::event_loop::EventLoop::with_user_event()
                .build()
                .expect("error creating event_loop.");

        let proxy = event_loop.create_proxy();

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

        {
            let mut cx = self.app.borrow_mut();
            cx.init_callback = Some(Box::new(on_init));
            cx.app_events.init(proxy);
        }

        event_loop
            .run_app(&mut self)
            .expect("Error running EventLoop");
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.app.borrow_mut().on_resumed(event_loop)
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.app.borrow_mut().on_about_to_wait(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        self.app.borrow_mut().on_user_event(event_loop, event)
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        self.jobs.run_foregound_tasks();
        self.app.borrow_mut().on_new_events(event_loop, cause)
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.app
            .borrow_mut()
            .on_window_event(event_loop, window_id, event);
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
pub type OpenWindowCallback = Box<dyn FnOnce(&mut Window, &mut AppContext) + 'static>;

pub struct AppContext {
    pub(crate) this: Weak<AppContextCell>,
    pub(crate) jobs: Jobs,
    init_callback: Option<AppInitCallback>,

    update_count: usize,
    running_effects: bool,
    effects: VecDeque<Effect>,
    pub(crate) app_events: AppEvents,

    #[allow(unused)]
    pub(crate) world: World,

    pending_user_events: ahash::AHashSet<UserEvent>,

    pub(crate) text_system: Arc<TextSystem>,

    pub(crate) texture_atlas: Arc<SkieAtlas>,

    pub(crate) windows: ahash::AHashMap<WindowId, Option<Window>>,

    pub(crate) gpu: GpuContext,
}

impl AppContext {
    fn new() -> AppContextRef {
        let jobs = Jobs::new(Some(7));

        let gpu = pollster::block_on(GpuContext::new()).expect("Error creating gpu context");

        let texture_system = Arc::new(SkieAtlas::new(gpu.clone()));

        let text_system = TextSystem::default();

        Rc::new_cyclic(|this| {
            RefCell::new(Self {
                this: this.clone(),
                jobs,
                init_callback: None,
                gpu,

                world: World::new(),

                update_count: 0,
                running_effects: false,
                effects: Default::default(),
                app_events: Default::default(),
                pending_user_events: Default::default(),

                texture_atlas: texture_system,
                text_system: Arc::new(text_system),
                windows: ahash::AHashMap::new(),
            })
        })
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

    fn run_effects(&mut self) {
        self.running_effects = true;

        while let Some(effect) = self.effects.pop_front() {
            match effect {
                Effect::AppEvent(event) => self.app_events.notify(event),
            }
        }

        self.running_effects = false;
    }

    pub(crate) fn update<R>(&mut self, update: impl FnOnce(&mut Self) -> R) -> R {
        self.update_count += 1;

        let res = update(self);

        if !self.running_effects && self.update_count == 1 {
            self.run_effects();
        }

        self.update_count -= 1;

        res
    }

    pub fn update_entity<T: 'static, R>(
        &mut self,
        handle: &Entity<T>,
        update: impl FnOnce(&mut T, &mut AppContext) -> R,
    ) -> R {
        self.update(|this| {
            let mut entity = this.world.detach(handle);
            let res = update(&mut entity, this);
            entity.reattach(&mut this.world);
            res
        })
    }

    pub(crate) fn push_effect(&mut self, effect: Effect) {
        match effect {
            Effect::AppEvent(event) => {
                if !self.pending_user_events.insert(event) {
                    return;
                }
            }
        }
        self.effects.push_back(effect);
    }

    pub(crate) fn push_app_event(&mut self, event: AppUpdateEvent) {
        self.app_events.push_event(event);
        self.push_effect(Effect::AppEvent(UserEvent::AppUpdate))
    }

    pub fn entity<T: 'static>(&mut self, value: T) -> Entity<T> {
        self.world.insert(value)
    }

    pub fn open_window<F>(&mut self, specs: WindowSpecification, on_load: F)
    where
        F: FnOnce(&mut Window, &mut AppContext) + 'static,
    {
        self.push_app_event(AppUpdateEvent::CreateWindow {
            specs,
            callback: Box::new(on_load),
        });
    }

    fn handle_window_create_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        specs: WindowSpecification,
        callback: OpenWindowCallback,
    ) {
        log::trace!("Creating window. \n Spec: {:#?}", &specs);

        self.update(|app| {
            match Window::new(
                event_loop,
                &specs,
                app.gpu.clone(),
                app.texture_atlas.clone(),
                app.text_system.clone(),
            ) {
                Ok(mut window) => {
                    callback(&mut window, app);
                    app.windows.insert(window.id(), Some(window));
                }
                Err(err) => log::error!("Error creating window\n{:#?}", err),
            };
        });
    }

    pub fn quit(&mut self) {
        self.update(|app| {
            app.push_effect(Effect::AppEvent(UserEvent::Quit));
        });
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

    pub fn update_window<R, Update>(&mut self, id: &WindowId, update: Update) -> Result<R>
    where
        Update: FnOnce(&mut Window, &mut Self) -> R,
    {
        self.update(|cx| {
            let mut window = cx
                .windows
                .get_mut(id)
                .ok_or(anyhow::anyhow!("window not found"))?
                .take()
                .ok_or(anyhow::anyhow!("window not found"))?;

            let res = update(&mut window, cx);

            cx.windows
                .get_mut(id)
                .ok_or(anyhow::anyhow!("window not found"))?
                .replace(window);

            Ok(res)
        })
    }

    fn handle_app_update_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        for event in self.app_events.drain() {
            match event {
                AppUpdateEvent::CreateWindow { specs, callback } => {
                    self.handle_window_create_event(event_loop, specs, callback);
                }
            }
        }
    }

    // BEGIN: WinitEventHandlers

    fn on_about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {}

    fn on_new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {}

    fn on_user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        self.pending_user_events.remove(&event);

        match event {
            UserEvent::AppUpdate => self.handle_app_update_event(event_loop),
            UserEvent::Quit => {
                event_loop.exit();
                self.app_events.dispose();
                log::info!("Bye!");
            }
        }
    }

    fn on_resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        log::info!("Initializing App...");
        self.update(|app| {
            if let Some(cb) = app.init_callback.take() {
                cb(app);
            }
        });
        log::info!("App Initialized!");
    }

    fn on_window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => {
                let width = size.width;
                let height = size.height;
                let _ = self.update_window(&window_id, |window, _| {
                    window.handle_resize(width, height);
                });
            }
            WindowEvent::RedrawRequested => {
                let _ = self.update_window(&window_id, |window, this| {
                    if let Err(error) = window.paint(this) {
                        log::error!("Error rendering {:#?}", error);
                    }
                });
            }
            WindowEvent::CursorMoved { .. } => {
                // todo
            }
            WindowEvent::MouseWheel { .. } => {
                // todo
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
                // TODO: do this in window update
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

    // END: WinitEventHandlers
}
