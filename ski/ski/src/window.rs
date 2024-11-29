pub mod error;

use std::sync::Arc;

use error::CreateWindowError;
pub(crate) use winit::window::Window as WinitWindow;

use crate::app::AppContext;

use ski_draw::{
    gpu::GpuContext,
    paint::{quad, TextureId, WgpuTexture},
    scene::Scene,
    WgpuRenderer, WgpuRendererSpecs,
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
    pub(crate) renderer: WgpuRenderer,
    pub(crate) handle: Arc<WinitWindow>,
    pub(crate) scene: Scene,

    // FIXME will be removed after adding atlas
    #[allow(unused)]
    thing_texture: WgpuTexture,

    #[allow(unused)]
    thing_texture_id: TextureId,

    #[allow(unused)]
    checker_texture: WgpuTexture,
    checker_texture_id: TextureId,
}

impl Window {
    pub(crate) fn new(
        event_loop: &winit::event_loop::ActiveEventLoop,
        gpu: Arc<GpuContext>,
        specs: &WindowSpecification,
    ) -> Result<Self, CreateWindowError> {
        let width = specs.width;
        let height = specs.height;

        let attr = winit::window::WindowAttributes::default()
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
            .with_title(specs.title);

        let winit_window = event_loop.create_window(attr).map_err(CreateWindowError)?;
        let handle = Arc::new(winit_window);

        // FIXME remove after adding atlas
        let thing_texture = load_thing(&gpu);

        let checker_data = create_checker_texture(250, 250, 25);
        let checker_texture =
            gpu.create_texture_init(wgpu::TextureFormat::Rgba8UnormSrgb, 250, 250, &checker_data);

        let mut renderer = WgpuRenderer::windowed(
            gpu,
            Arc::clone(&handle),
            &WgpuRendererSpecs { width, height },
        )
        .unwrap();

        let checker_texture_id = renderer.set_native_texture(
            &checker_texture.create_view(&wgpu::TextureViewDescriptor::default()),
        );

        let thing_texture_id = renderer.set_native_texture(
            &thing_texture.create_view(&wgpu::TextureViewDescriptor::default()),
        );

        Ok(Self {
            scene: Scene::default(),
            handle,
            renderer,
            thing_texture,
            thing_texture_id,
            checker_texture,
            checker_texture_id,
        })
    }

    pub fn set_bg_color(&mut self, r: f64, g: f64, b: f64) {
        let color = wgpu::Color { r, g, b, a: 1.0 };
        self.renderer.set_clear_color(color);
        self.handle.request_redraw();
    }

    #[inline]
    pub fn id(&self) -> winit::window::WindowId {
        self.handle.id()
    }

    pub(crate) fn handle_resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    pub fn winit_handle(&self) -> &Arc<WinitWindow> {
        &self.handle
    }

    // FIXME for now
    pub fn build_scene(&mut self) {
        self.scene.clear();

        let size = self.winit_handle().inner_size();
        let width = size.width as f32;
        let height = size.height as f32;

        self.scene.add(
            quad()
                .with_pos(width / 2.0 - 150.0, height / 2.0 - 150.0)
                .with_size(300.0, 300.0)
                .with_bgcolor(1.0, 0.0, 0.0, 1.0),
            Some(self.checker_texture_id),
        );

        self.scene.add(
            quad()
                .with_pos(100.0, 200.0)
                .with_size(250.0, 250.0)
                .with_bgcolor(1.0, 1.0, 0.0, 1.0),
            Some(self.checker_texture_id),
        );

        self.scene.add(
            quad()
                .with_pos(width / 2.0 + 300.0, 400.0)
                .with_size(500.0, 500.0),
            Some(self.thing_texture_id),
        );

        self.scene.add(
            quad()
                .with_pos(100.0, 500.0)
                .with_size(300.0, 100.0)
                .with_bgcolor(0.3, 0.3, 0.9, 1.0),
            None,
        );

        let bar_height: f32 = 50.0;
        let margin_bottom: f32 = 30.0;

        self.scene.add(
            quad()
                .with_pos(0.0, height - bar_height - margin_bottom)
                .with_size(width, bar_height)
                .with_bgcolor(0.04, 0.04, 0.07, 1.0),
            None,
        );
    }

    pub(crate) fn paint(&mut self) {
        // FIXME for now
        self.build_scene();

        let batches = self.scene.batches().collect::<Vec<_>>();

        self.renderer.update_buffers(&batches);

        self.renderer.render(&batches);
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
}

fn create_checker_texture(width: usize, height: usize, tile_size: usize) -> Vec<u8> {
    let mut texture_data = vec![0u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let tile_x = x / tile_size;
            let tile_y = y / tile_size;
            let is_black = (tile_x + tile_y) % 2 == 0;

            let offset = (y * width + x) * 4;
            if is_black {
                texture_data[offset] = 0; // Red
                texture_data[offset + 1] = 0; // Green
                texture_data[offset + 2] = 0; // Blue
                texture_data[offset + 3] = 255; // Alpha
            } else {
                texture_data[offset] = 255; // Red
                texture_data[offset + 1] = 255; // Green
                texture_data[offset + 2] = 255; // Blue
                texture_data[offset + 3] = 255; // Alpha
            }
        }
    }

    texture_data
}

fn load_thing(gpu: &GpuContext) -> WgpuTexture {
    let thing_buffer = include_bytes!("../../../assets/thing2.png");

    let thing = image::load_from_memory(thing_buffer).unwrap();
    let data = thing.into_rgba8();

    // FIXME color
    gpu.create_texture_init(
        ski_draw::paint::TextureFormat::Rgba8UnormSrgb,
        data.width(),
        data.height(),
        &data,
    )
}
