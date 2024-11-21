use std::cell::RefCell;
use std::rc::Rc;

pub mod context;
pub use context::AppContext;

use crate::window::{WindowContext, WindowId, WindowSpecification};

use winit::application::ApplicationHandler;
use winit::event_loop::EventLoop;

use parking_lot::Mutex;

pub(crate) static EVENT_LOOP_PROXY: Mutex<Option<winit::event_loop::EventLoopProxy<UserEvent>>> =
    Mutex::new(None);

pub type InitCallback = Box<dyn FnOnce(&mut AppContext) + 'static>;
pub type OpenWindowCallback = Box<dyn FnOnce(&mut WindowContext) + 'static>;

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

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

        let proxy = event_loop.create_proxy();

        *EVENT_LOOP_PROXY.lock() = Some(proxy);

        if let Err(err) = event_loop.run_app(self) {
            println!("Error running app: Error: {}", err);
        } else {
            EVENT_LOOP_PROXY.lock().take();
        };
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
    AppContextCallback {
        callback: Box<dyn FnOnce(&mut AppContext) + 'static>,
    },
    #[allow(unused)]
    Other,
}

pub(crate) enum Effect {
    UserEvent(UserEvent),

    #[allow(unused)]
    Other,
}
