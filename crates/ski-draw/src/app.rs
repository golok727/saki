pub mod context;
pub use context::AppContext;

pub(crate) mod events;

use crate::window::{WindowContext, WindowId, WindowSpecification};

use winit::event_loop::EventLoop;

use parking_lot::Mutex;

pub(crate) static EVENT_LOOP_PROXY: Mutex<Option<winit::event_loop::EventLoopProxy<AppAction>>> =
    Mutex::new(None);

pub type InitCallback = Box<dyn FnOnce(&mut AppContext) + 'static>;
pub type OpenWindowCallback = Box<dyn FnOnce(&mut WindowContext) + 'static>;

pub struct App(AppContext);

impl App {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(AppContext::new())
    }

    pub fn run<F>(&mut self, f: F)
    where
        F: FnOnce(&mut AppContext) + 'static,
    {
        self.0.init_callback = Some(Box::new(f));

        let event_loop: EventLoop<AppAction> = EventLoop::with_user_event()
            .build()
            .expect("error creating event_loop.");

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

        let proxy = event_loop.create_proxy();

        *EVENT_LOOP_PROXY.lock() = Some(proxy);

        if let Err(err) = event_loop.run_app(&mut self.0) {
            println!("Error running app: Error: {}", err);
        } else {
            EVENT_LOOP_PROXY.lock().take();
        };
    }
}

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
    ChangeWindowBg {
        window_id: WindowId,
        color: (f64, f64, f64),
    },
    AppContextCallback {
        callback: Box<dyn FnOnce(&mut AppContext) + 'static>,
    },
}

pub(crate) enum Effect {
    UserEvent(AppAction),
}
