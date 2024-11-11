use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use saki_draw::{GpuContext, Renderer, SurfaceRenderTarget, SurfaceRenderTargetSpecs};

use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowAttributes,
};

type InitCallback = Box<dyn FnOnce(&mut App) + 'static>;
type FrameCallback = Box<dyn FnOnce(&mut App) + 'static>;

type GpuRefCell = Rc<RefCell<GpuContext>>;

struct App {
    init_callback: Option<InitCallback>,
    frame_callbacks: Rc<RefCell<Vec<FrameCallback>>>,

    window: Option<Arc<winit::window::Window>>,
    gpu_context: Option<GpuRefCell>,
}

impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            init_callback: None,
            gpu_context: None,
            frame_callbacks: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub async fn init(&mut self) {
        let cx = GpuContext::new().await;
        self.gpu_context = Some(Rc::new(RefCell::new(cx)));
    }

    pub fn on_next_frame<F>(&mut self, f: F)
    where
        F: FnOnce(&mut App) + 'static,
    {
        RefCell::borrow_mut(&self.frame_callbacks).push(Box::new(f))
    }

    pub fn run<F>(&mut self, f: F)
    where
        F: FnOnce(&mut App) + 'static,
    {
        self.init_callback = Some(Box::new(f));
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        if let Err(err) = event_loop.run_app(self) {
            println!("Error running app: Error: {}", err);
        };
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        println!("resumed!");
        if self.window.is_none() {
            let attr = WindowAttributes::default()
                .with_inner_size(winit::dpi::PhysicalSize {
                    width: 1280,
                    height: 920,
                })
                .with_title("Saki");

            let window = event_loop.create_window(attr).unwrap();

            self.window = Some(Arc::new(window));

            if let Some(cb) = self.init_callback.take() {
                cb(self);
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
                if let Some(window) = &self.window {
                    // TODO add wm and close window only
                    if window.id() == window_id {
                        event_loop.exit();
                        println!("Bye!");
                    }
                }
            }
            _ => {
                //
            }
        }
    }
}

fn main() {
    println!("Radhe Shyam!");

    let mut app = App::new();

    async_std::task::block_on(app.init());

    app.run(|app| {
        if let Some(context) = &app.gpu_context {
            let gpu = context.clone();

            let window = app.window.as_ref().expect("window_not_found");
            let size = window.inner_size();

            let specs = &SurfaceRenderTargetSpecs {
                width: size.width,
                height: size.height,
            };

            let surface_target = {
                let mut gpu = gpu.borrow_mut();
                let screen = Arc::clone(window);
                SurfaceRenderTarget::new(specs, &mut gpu, screen)
            };

            let renderer = Rc::new(RefCell::new(Renderer::new(gpu, surface_target)));

            let ren = Rc::clone(&renderer);

            app.on_next_frame(move |_| {
                let mut renderer = ren.borrow_mut();
                renderer.render();
            })
        }
    });
}
