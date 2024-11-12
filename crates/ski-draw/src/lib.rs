use std::sync::Arc;

pub mod app;
pub mod gpu;

use gpu::{surface::GpuSurface, GpuContext};

pub trait RenderTarget: std::fmt::Debug + 'static {
    fn get_texture(&self) -> &wgpu::Texture;

    fn get_view(&self) -> wgpu::TextureView;

    fn resize(&mut self, width: u32, height: u32);

    fn update(&mut self, gpu: &GpuContext);

    fn prerender(&mut self) {}

    fn postrender(&mut self) {}
}

impl RenderTarget for GpuSurface {
    fn update(&mut self, gpu: &GpuContext) {
        self.sync(gpu);
    }

    fn get_view(&self) -> wgpu::TextureView {
        self.create_view()
    }

    fn postrender(&mut self) {
        self.present();
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.resize(width, height)
    }

    fn get_texture(&self) -> &wgpu::Texture {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct Renderer {
    render_target: Box<dyn RenderTarget>,
    gpu: Arc<GpuContext>,
}

impl Renderer {
    pub fn new<T>(gpu: Arc<GpuContext>, target: T) -> Self
    where
        T: RenderTarget,
    {
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

        let gpu = self.gpu.as_ref();

        self.render_target.update(gpu);

        log::info!("prerender");
        self.render_target.prerender();

        let view = self.render_target.get_view();

        let mut encoder = gpu.device.create_command_encoder(
            &(wgpu::CommandEncoderDescriptor {
                label: Some("my encoder"),
            }),
        );

        {
            let _render_pass = encoder.begin_render_pass(
                &(wgpu::RenderPassDescriptor {
                    label: Some("Render pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                }),
            );
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
        log::info!("render");

        log::info!("postrender");
        self.render_target.postrender();

        log::info!("Rendering things!");
    }
}
