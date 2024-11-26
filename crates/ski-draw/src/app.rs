pub mod context;
use std::hash::Hash;

pub use context::AppContext;

pub(crate) mod events;

use crate::{
    paint::quad,
    scene::Scene,
    window::{WindowContext, WindowId, WindowSpecification},
};

use winit::event_loop::EventLoop;

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

        self.0.app_events.init(proxy.clone());

        if let Err(err) = event_loop.run_app(&mut self.0) {
            println!("Error running app: Error: {}", err);
        } else {
            self.0.app_events.dispose();
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

// TODO remove
#[allow(unused)]
fn batch_test() {
    let mut scene = Scene::default();

    let width = 100.0;
    let height = 200.0;

    scene.add(
        quad()
            .with_pos((width / 2.) - 100.0, (height / 2.) - 100.0)
            .with_size(200., 200.)
            .with_bgcolor(1., 0., 0., 1.), // green,
        None,
    );

    scene.add(
        quad()
            .with_pos(100.0, 200.0)
            .with_size(250., 250.)
            .with_bgcolor(0., 1., 0., 1.), // green,
        None,
    );

    scene.add(
        quad()
            .with_pos(100.0, 500.0)
            .with_size(300.0, 100.0)
            .with_bgcolor(0.3, 0.3, 0.9, 1.0),
        Some(crate::paint::TextureId::User(1)),
    );

    let bar_height: f32 = 50.0;
    let margin_bottom: f32 = 30.0;

    scene.add(
        quad()
            .with_pos(0.0, (height - bar_height) - margin_bottom)
            .with_size(width, bar_height)
            .with_bgcolor(0.04, 0.04, 0.07, 1.0),
        Some(crate::paint::TextureId::User(2)),
    );

    let thing = scene.batches().collect::<Vec<_>>();

    dbg!(thing.len());
}
