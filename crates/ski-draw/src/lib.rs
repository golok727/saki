pub mod gpu;
pub mod app;
use std::{ cell::RefCell, rc::Rc };

pub use gpu::GpuContext;

pub trait RenderTarget: std::fmt::Debug + 'static {
    fn get_texture(&self) -> &wgpu::Texture;

    fn get_view(&self) -> wgpu::TextureView;

    fn resize(&mut self, width: u32, height: u32);

    fn update(&mut self, gpu: &mut GpuContext);

    fn prerender(&mut self) {}

    fn postrender(&mut self) {}
}

#[derive(Debug)]
pub struct SurfaceRenderTargetSpecs {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct SurfaceRenderTarget {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    cur_texture: Option<wgpu::SurfaceTexture>,
    dirty: bool,
}

impl SurfaceRenderTarget {
    pub fn new(
        specs: &SurfaceRenderTargetSpecs,
        gpu: &mut GpuContext,
        screen: impl Into<wgpu::SurfaceTarget<'static>>
    ) -> Self {
        let width = specs.width.max(1);
        let height = specs.height.max(1);

        let instance = &gpu.instance;
        let surface = instance.create_surface(screen).unwrap();

        let capabilities = surface.get_capabilities(&gpu.adapter);

        let surface_format = capabilities.formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(capabilities.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: capabilities.present_modes[0],
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&gpu.device, &surface_config);

        Self {
            surface,
            surface_config,
            cur_texture: None,
            dirty: false,
        }
    }
}

impl RenderTarget for SurfaceRenderTarget {
    fn update(&mut self, gpu: &mut GpuContext) {
        if self.dirty {
            self.surface.configure(&gpu.device, &self.surface_config);
            self.dirty = false;
        }
    }

    fn get_texture(&self) -> &wgpu::Texture {
        let tex = self.cur_texture.as_ref().unwrap();
        &tex.texture
    }

    fn get_view(&self) -> wgpu::TextureView {
        let texture = self.get_texture();
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn prerender(&mut self) {
        if self.cur_texture.is_none() {
            let surface_texture = self.surface.get_current_texture().unwrap();
            self.cur_texture = Some(surface_texture);
        }
    }

    fn postrender(&mut self) {
        if let Some(texture) = self.cur_texture.take() {
            texture.present();
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.dirty = true;
    }
}

#[derive(Debug)]
pub struct Renderer {
    render_target: Box<dyn RenderTarget>,
    gpu: Rc<RefCell<GpuContext>>,
}

impl Renderer {
    pub fn new<T>(gpu: Rc<RefCell<GpuContext>>, target: T) -> Self where T: RenderTarget {
        Self {
            gpu,
            render_target: Box::new(target),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.render_target.resize(width, height);
    }

    pub fn rect(&mut self) {
        todo!()
    }

    pub fn circle(&mut self) {
        todo!()
    }

    pub fn render(&mut self) {
        let (r, g, b, a) = (1.0, 1.0, 0.0, 1.0);

        let mut gpu = self.gpu.borrow_mut();

        self.render_target.update(&mut gpu);

        log::info!("prerender");
        self.render_target.prerender();

        let view = self.render_target.get_view();

        let mut encoder = gpu.device().create_command_encoder(
            &(wgpu::CommandEncoderDescriptor {
                label: Some("my encoder"),
            })
        );

        {
            let _render_pass = encoder.begin_render_pass(
                &(wgpu::RenderPassDescriptor {
                    label: Some("Render pass"),
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                                store: wgpu::StoreOp::Store,
                            },
                        }),
                    ],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                })
            );
        }

        gpu.queue().submit(std::iter::once(encoder.finish()));
        log::info!("render");

        log::info!("postrender");
        self.render_target.postrender();

        log::info!("Rendering things!");
    }
}
