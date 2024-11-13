use std::sync::Arc;

mod render_target;

pub use render_target::MockRenderTarget;

use render_target::{RenderTarget, RenderTargetSpecification};

use crate::gpu::GpuContext;

#[derive(Debug)]
pub struct Renderer {
    // TODO remove
    mock_render_target: Box<dyn MockRenderTarget>,

    render_target: RenderTarget,

    gpu: Arc<GpuContext>,
}

impl Renderer {
    pub fn new<T>(gpu: Arc<GpuContext>, target: T, width: u32, height: u32) -> Self
    where
        T: MockRenderTarget,
    {
        let render_target_spec = RenderTargetSpecification::default().with_size(width, height);

        let render_target = RenderTarget::new(&gpu, &render_target_spec);

        Self {
            gpu,
            render_target,
            mock_render_target: Box::new(target),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.mock_render_target.resize(width, height);

        self.render_target.resize(&self.gpu, width, height);
    }

    pub fn destroy(&mut self) {}

    pub fn render(&mut self) {
        let (r, g, b, a) = (1.0, 1.0, 0.0, 1.0);

        let gpu = &self.gpu;

        self.mock_render_target.update(gpu);

        log::info!("prerender");
        self.mock_render_target.prerender();

        let view = self.mock_render_target.get_view();

        let mut encoder = gpu.create_command_encoder(Some("my encoder"));

        {
            encoder.begin_render_pass(
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

        self.mock_render_target.postrender();

        log::info!("Rendering things!");
    }
}
