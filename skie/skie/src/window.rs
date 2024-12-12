pub mod error;

use core::f32;
use std::{future::Future, io::Read, sync::Arc};

use error::CreateWindowError;
use image::ImageBuffer;
pub(crate) use winit::window::Window as WinitWindow;

use crate::{
    app::{AppContext, AsyncAppContext},
    jobs::Job,
};

use skie_draw::{
    gpu::GpuContext,
    math::{unit::Pixels, Rect, Size},
    paint::{atlas::AtlasManager, quad, TextureId, TextureKind},
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
            title: "skie",
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

// for playing around remove it later
#[derive(Debug, Clone)]
enum Object {
    Image {
        bbox: Rect<Pixels>,
        natural_width: f32,
        natural_height: f32,
        texture: TextureId,
    },
}

#[derive(Debug)]
pub struct Window {
    pub(crate) renderer: WgpuRenderer,
    pub(crate) handle: Arc<WinitWindow>,
    pub(crate) scene: Scene,

    objects: Vec<Object>,

    yellow_thing_texture_id: TextureId,
    checker_texture_id: TextureId,

    #[allow(unused)]
    pub(crate) texture_system: AtlasManager,
    next_texture_id: usize,
}

impl Window {
    pub(crate) fn new(
        event_loop: &winit::event_loop::ActiveEventLoop,
        gpu: Arc<GpuContext>,
        texture_system: AtlasManager,
        specs: &WindowSpecification,
    ) -> Result<Self, CreateWindowError> {
        #[cfg(never)]
        {
            use skie_draw::{
                math::{unit::px, vec2},
                paint::{Path, PathData},
            };

            let mut path = Path::<Pixels>::default();
            path.move_to((px(100.0), px(100.0)).into());
            path.line_to((px(200.0), px(100.0)).into());

            path.arc(
                vec2(px(300.0), px(300.0)),
                px(100.0),
                0.0,
                f32::consts::TAU as f64,
                false,
            );

            let data = PathData::from(&mut path);
            let mut python_array = String::from("[\n");
            for point in &data.points {
                python_array.push_str(&format!("  [{}, {}],\n", point.x, point.y));
            }
            python_array.push(']');
            println!("{}", python_array);
        }
        // END _TEST

        let width = specs.width;
        let height = specs.height;

        let attr = winit::window::WindowAttributes::default()
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
            .with_title(specs.title);

        let winit_window = event_loop.create_window(attr).map_err(CreateWindowError)?;
        let handle = Arc::new(winit_window);

        let mut renderer = WgpuRenderer::windowed(
            gpu,
            texture_system.clone(),
            Arc::clone(&handle),
            &(WgpuRendererSpecs { width, height }),
        )
        .unwrap();

        let checker_texture_id = TextureId::User(1001);
        let yellow_thing_texture_id = TextureId::User(1002);

        let checker_data = create_checker_texture(250, 250, 25);

        texture_system.get_or_insert(&checker_texture_id, || {
            (
                TextureKind::Color,
                Size {
                    width: 250.into(),
                    height: 250.into(),
                },
                &checker_data,
            )
        });

        let thing_data = load_thing();
        texture_system.get_or_insert(&yellow_thing_texture_id, || {
            (
                TextureKind::Color,
                Size {
                    width: thing_data.width().into(),
                    height: thing_data.height().into(),
                },
                &thing_data,
            )
        });

        renderer.set_atlas_texture(&checker_texture_id);
        renderer.set_atlas_texture(&yellow_thing_texture_id);

        Ok(Self {
            scene: Scene::default(),
            handle,
            renderer,
            texture_system,
            yellow_thing_texture_id,
            checker_texture_id,
            objects: Vec::new(),
            // FIXME: this is bad
            next_texture_id: 10000,
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

    // FIXME: for now
    pub fn build_scene(&mut self) {
        self.scene.clear();

        let size = self.winit_handle().inner_size();
        let width = size.width as f32;
        let height = size.height as f32;

        self.scene.add_textured(
            quad()
                .with_pos(width / 2.0 - 350.0, height / 2.0 - 350.0)
                .with_size(700.0, 700.0),
            self.yellow_thing_texture_id,
        );

        self.scene.add_textured(
            quad()
                .with_pos(100.0, height - 400.0)
                .with_size(300.0, 300.0)
                .with_bgcolor(1.0, 0.0, 0.0, 1.0),
            self.checker_texture_id,
        );

        self.scene.add_textured(
            quad()
                .with_pos(100.0, 200.0)
                .with_size(250.0, 250.0)
                .with_bgcolor(1.0, 1.0, 0.0, 1.0),
            self.checker_texture_id,
        );

        self.scene.add_textured(
            quad()
                .with_pos(width - 300.0, height - 300.0)
                .with_size(200.0, 200.0),
            self.yellow_thing_texture_id,
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
                .with_pos(0.0, height - bar_height - margin_bottom)
                .with_size(width, bar_height)
                .with_bgcolor(0.04, 0.04, 0.07, 1.0),
        );

        for object in &self.objects {
            match object {
                Object::Image {
                    bbox,
                    texture,
                    natural_width,
                    natural_height,
                } => {
                    let aspect = natural_width / natural_height;
                    let x: f32 = bbox.x.into();
                    let y: f32 = bbox.y.into();
                    let width: f32 = (bbox.width * aspect).into();
                    let height: f32 = (bbox.height).into();

                    self.scene
                        .add_textured(quad().with_pos(x, y).with_size(width, height), *texture);
                }
            }
        }
    }

    pub(crate) fn paint(&mut self) {
        self.build_scene();

        let info_map = self
            .texture_system
            .get_texture_infos(self.scene.get_required_textures());

        let batches = self.scene.batches(info_map).collect::<Vec<_>>();

        self.renderer.update_buffers(&batches);

        self.renderer.render(&batches);
    }

    fn get_next_tex_id(&mut self) -> TextureId {
        let id = self.next_texture_id;
        self.next_texture_id += 1;
        TextureId::User(id)
    }

    pub(crate) fn refresh(&self) {
        self.handle.request_redraw();
    }
}

pub struct AsyncWindowContext {
    app: AsyncAppContext,
    window_id: WindowId,
}

impl AsyncWindowContext {
    pub fn with<R>(&self, reader: impl FnOnce(&mut WindowContext) -> R) -> Option<R> {
        let app = self.app.app.upgrade().expect("app released");
        let mut lock = app.borrow_mut();
        let window = lock.windows.remove(&self.window_id);

        if let Some(mut window) = window {
            let mut cx = WindowContext::new(&mut lock, &mut window);
            let res = reader(&mut cx);
            lock.windows.insert(window.id(), window);
            Some(res)
        } else {
            None
        }
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

    pub fn to_async(&self) -> AsyncWindowContext {
        AsyncWindowContext {
            app: self.app.to_async(),
            window_id: self.window.id(),
        }
    }

    pub fn spawn<Fut, R>(&self, f: impl FnOnce(AsyncWindowContext) -> Fut) -> Job<R>
    where
        Fut: Future<Output = R> + 'static,
        R: 'static,
    {
        let cx = self.to_async();
        self.app.jobs.spawn(f(cx))
    }

    pub fn load_image_from_file(&mut self, rect: Rect<Pixels>, file_path: String) {
        // TODO: better error
        let img_job = self.app.jobs.spawn_blocking({
            let file_path = file_path.clone();

            async move {
                let file = std::fs::File::open(file_path);
                if file.is_err() {
                    log::error!("Error reading image file");
                    return None;
                }

                let mut file = file.unwrap();
                let mut data = Vec::<u8>::new();
                if file.read_to_end(&mut data).is_err() {
                    log::error!("Error reading image file");
                    return None;
                }

                let loaded_image = image::load_from_memory(&data);

                if loaded_image.is_err() {
                    log::error!("Error loading image file");
                    return None;
                }

                Some(loaded_image.unwrap().to_rgba8())
            }
        });

        self.spawn(|cx| async move {
            let img = img_job.await;
            if img.is_none() {
                return;
            }
            let img = img.unwrap();

            let width = img.width();
            let height = img.height();

            cx.with(|cx| {
                let id = cx.window.get_next_tex_id();
                cx.window.texture_system.get_or_insert(&id, || {
                    (
                        TextureKind::Color,
                        Size {
                            width: width.into(),
                            height: height.into(),
                        },
                        &img,
                    )
                });

                cx.window.renderer.set_atlas_texture(&id);
                cx.window.objects.push(Object::Image {
                    bbox: rect,
                    natural_width: width as f32,
                    natural_height: height as f32,
                    texture: id,
                });
                // FIXME: maybe mark window as dirty instead and allow the app to handle this ?
                cx.window.refresh();
            });
        })
        .detach();
    }

    pub fn set_timeout(
        &mut self,
        f: impl FnOnce(&mut WindowContext) + 'static,
        timeout: std::time::Duration,
    ) {
        self.spawn(|cx| async move {
            cx.app.jobs.timer(timeout).await;
            cx.with(|cx| f(cx));
        })
        .detach();
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

fn load_thing() -> ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let thing_buffer = include_bytes!("../../../assets/thing2.png");

    let thing = image::load_from_memory(thing_buffer).unwrap();
    thing.into_rgba8()
}
