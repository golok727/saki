use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;

use crate::gpu::GpuContext;
use crate::jobs::{Job, Jobs};
use crate::window::error::CreateWindowError;
use crate::window::{Window, WindowContext, WindowId, WindowSpecification};

use winit::application::ApplicationHandler;
use winit::{
    event::{KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use super::{AppAction, AppUpdateEvent, Effect, InitCallback, OpenWindowCallback, EVENT_LOOP_PROXY};


pub struct AppContext {
    pub(super) init_callback: Option<InitCallback>,

    pub(crate) jobs: Jobs,

    pending_updates: usize,
    flushing_effects: bool,
    pending_user_events: HashSet<AppAction>,
    effects: VecDeque<Effect>,
    app_events: Rc<RefCell<Vec<AppUpdateEvent>>>,

    windows: HashMap<WindowId, Window>,
    pub(crate) gpu: Arc<GpuContext>,
}


impl AppContext {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        // TODO handle error
        let gpu = pollster::block_on(GpuContext::new()).unwrap();

        // FIXME
        let jobs = Jobs::new(Some(7));

        Self {
            init_callback: None,

            app_events: Default::default(),
            pending_user_events: HashSet::new(),

            pending_updates: 0,
            flushing_effects: false,
            effects: VecDeque::new(),

            jobs,

            windows: HashMap::new(),
            gpu: Arc::new(gpu),
        }
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

    fn notify_user_event(&self, event: AppAction) {
        AppContext::use_proxy(|proxy| {
            if let Err(error) = proxy.send_event(event) {
                log::error!("Error sending: {}", &error)
            }
        });
    }

    pub(crate) fn push_app_event(&mut self, event: AppUpdateEvent) {
        RefCell::borrow_mut(&self.app_events).push(event);
        self.push_effect(Effect::UserEvent(AppAction::AppUpdate))
    }

    pub fn quit(&mut self) {
        self.update(|app| {
            app.push_effect(Effect::UserEvent(AppAction::Quit));
        })
    }

    // todo will be removed
    pub fn change_bg(&mut self, window_id: WindowId, color: (f64, f64, f64)) {
        self.update(|app| {
            app.push_app_event(AppUpdateEvent::ChangeWindowBg { window_id, color });
        })
    }


    pub fn spawn<T>(&self, future: impl Future<Output = T> + 'static) -> Job<T>
    where
        T: 'static {
        self.jobs.spawn_local(future)
    }

    pub fn spawn_bg<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Job<T>
    where
        T: Send + 'static {
        self.jobs.spawn(future)
    }

    pub fn set_timeout(
        &mut self,
        f: impl FnOnce(&mut Self) + 'static,
        timeout: std::time::Duration,
    ) {
        let jobs = self.jobs.clone();
        let events = Rc::clone(&self.app_events); 

        self.
            spawn(async move {
                jobs.timer(timeout).await;
                RefCell::borrow_mut(&events).push(AppUpdateEvent::AppContextCallback {
                    callback: Box::new(f),
                });
                AppContext::use_proxy(|proxy| {
                    let _ = proxy.send_event(AppAction::AppUpdate);
                })
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
        self.push_app_event(AppUpdateEvent::CreateWindow {
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
                 AppUpdateEvent::CreateWindow { specs, callback } => {
                    self.handle_window_create_event(event_loop, specs, callback);
                }
                AppUpdateEvent::ChangeWindowBg { window_id, color } => {
                    if let Some(window) = self.windows.get_mut(&window_id) {
                        window.set_bg_color(color.0, color.1, color.2);
                    }
                }
                AppUpdateEvent::AppContextCallback { callback } => callback(self),
            }
        }
    }

    // please pass in the AppEvent::CreateWindow or it will panic
    fn handle_window_create_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        specs: WindowSpecification, 
        callback: OpenWindowCallback, 
    ) {
        log::info!("Creating window. \n Spec: {:#?}", &specs);
        if let Ok((id, mut window)) = self.create_window(&specs, event_loop) {
            let mut context = WindowContext::new(self, &mut window);

            callback(&mut context);

            let _ = self.windows.insert(id, window);
        } else {
            log::error!("Error creating window")
        }
    }

    pub(crate) fn use_proxy<F>(f: F)
    where
        F: FnOnce(&winit::event_loop::EventLoopProxy<AppAction>),
    {
        if let Some(proxy) = EVENT_LOOP_PROXY.lock().as_ref() {
            f(proxy)
        }
    }
}

impl ApplicationHandler<AppAction> for AppContext {
    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: AppAction) {
        self.pending_user_events.remove(&event);

        match event {
            AppAction::AppUpdate => self.handle_app_update_event(event_loop),
            AppAction::Quit => {
                event_loop.exit();
                EVENT_LOOP_PROXY.lock().take();
                log::info!("Bye!");
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

    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        log::info!("Initializing App...");
        if let Some(cb) = self.init_callback.take() {
            cb(self);
        }
        log::info!("App Initialized!");
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.jobs.run_foregound_tasks();
    }
}
