pub mod error;

use std::sync::Arc;

// use crate::gpu::surface::GpuSurface;
// use crate::renderer::Renderer;

use error::CreateWindowError;
pub(crate) use winit::window::Window as WinitWindow;

use crate::{
    app::AppContext,
    gpu::{
        surface::{GpuSurface, GpuSurfaceSpecification},
        GpuContext,
    },
    scene::{quad, Scene},
    Renderer,
};

#[derive(Debug, Clone)]
pub struct WindowSpecification {
    pub width: u32,
    pub height: u32,
    pub title: &'static str,
}

pub type WindowId = winit::window::WindowId;

impl Default for WindowSpecification {
    fn default() -> Self {
        Self {
            width: 800,
            height: 800,
            title: "Ski",
        }
    }
}

impl WindowSpecification {
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_title(mut self, title: &'static str) -> Self {
        self.title = title;
        self
    }
}

#[derive(Debug)]
pub struct Window {
    pub(crate) surface: GpuSurface,
    pub(crate) renderer: Renderer,
    pub(crate) handle: Arc<WinitWindow>,
    pub(crate) scene: Scene,
    bg_color: wgpu::Color,
}

impl Window {
    pub(crate) fn new(
        event_loop: &winit::event_loop::ActiveEventLoop,
        specs: &WindowSpecification,
        gpu: Arc<GpuContext>,
    ) -> Result<Self, CreateWindowError> {
        let width = specs.width;
        let height = specs.height;

        let attr = winit::window::WindowAttributes::default()
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
            .with_title(specs.title);

        let winit_window = event_loop.create_window(attr).map_err(CreateWindowError)?;
        let handle = Arc::new(winit_window);

        let surface = gpu
            .create_surface(
                Arc::clone(&handle),
                &GpuSurfaceSpecification { width, height },
            )
            .unwrap(); // TODO handle error

        let renderer = Renderer::new(gpu, width, height);

        Ok(Self {
            bg_color: wgpu::Color::WHITE,
            scene: Scene::default(),
            handle,
            renderer,
            surface,
        })
    }

    // for now :)
    pub fn set_bg_color(&mut self, r: f64, g: f64, b: f64) {
        self.bg_color = wgpu::Color { r, g, b, a: 1.0 };
        self.handle.request_redraw();
    }

    #[inline]
    pub fn id(&self) -> winit::window::WindowId {
        self.handle.id()
    }

    pub(crate) fn handle_resize(&mut self, width: u32, height: u32) {
        self.surface.resize(&self.renderer.gpu, width, height);
        self.renderer.resize(width, height);
    }

    pub fn winit_handle(&self) -> &Arc<WinitWindow> {
        &self.handle
    }

    // for now
    pub fn build_scene(&mut self) {
        self.scene.clear();

        let size = self.winit_handle().inner_size();
        let width = size.width as f32;
        let height = size.height as f32;

        self.scene.add(
            quad()
                .with_pos((width / 2.) - 100.0, (height / 2.) - 100.0)
                .with_size(200., 200.)
                .with_bgcolor(1., 0., 0., 1.), // green
        );

        self.scene.add(
            quad()
                .with_pos(100.0, 200.0)
                .with_size(250., 250.)
                .with_bgcolor(0., 1., 0., 1.), // green
        );

        self.scene.add(
            quad()
                .with_pos(100.0, 500.0)
                .with_size(300.0, 100.0)
                .with_bgcolor(0.3, 0.3, 0.9, 1.0),
        );

        let bar_height: f32 = 50.0;
        let margin_bottom: f32 = 30.0;

        self.scene.add(
            quad()
                .with_pos(0.0, (height - bar_height) - margin_bottom)
                .with_size(width, bar_height)
                .with_bgcolor(0.04, 0.04, 0.07, 1.0),
        );
    }

    pub(crate) fn paint(&mut self) {
        let surface_texture = self.surface.surface.get_current_texture().unwrap();

        self.build_scene();

        self.renderer
            .render_to_texture(self.bg_color, &self.scene, &surface_texture.texture);

        surface_texture.present();
    }
}

pub struct WindowContext<'a> {
    pub app: &'a mut AppContext,
    pub window: &'a mut Window,
}

impl<'a> WindowContext<'a> {
    pub fn new(app: &'a mut AppContext, window: &'a mut Window) -> Self {
        Self { app, window }
    }

    pub fn open_window<F>(&mut self, specs: WindowSpecification, f: F)
    where
        F: Fn(&mut WindowContext) + 'static,
    {
        self.app.open_window(specs, f)
    }

    pub fn set_timeout<F>(&mut self, f: F, timeout: std::time::Duration)
    where
        F: FnOnce(&mut WindowContext) + 'static,
    {
        let window_id = self.window.id();

        self.app.set_timeout(
            move |app| {
                app.update(|app| {
                    app.push_app_event(crate::app::AppUpdateEvent::WindowContextCallback {
                        callback: Box::new(f),
                        window_id,
                    });
                })
            },
            timeout,
        );
    }

    pub fn change_bg() {}
}
