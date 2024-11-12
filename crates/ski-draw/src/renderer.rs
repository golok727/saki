use std::sync::Arc;

mod render_target;
pub use render_target::RenderTarget;

use crate::gpu::GpuContext;

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

        let gpu = &self.gpu;

        self.render_target.update(gpu);

        log::info!("prerender");
        self.render_target.prerender();

        let view = self.render_target.get_view();

        let mut encoder = gpu.create_command_encoder(Some("my encoder"));

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
