use std::sync::Arc;

pub mod render_target;

use render_target::{RenderTarget, RenderTargetSpecification};

use crate::gpu::GpuContext;

#[derive(Debug, Default)]
pub struct Pipes {
    pub quad: Option<wgpu::RenderPipeline>,
}

impl Pipes {
    pub fn destroy(&mut self) {}
}

#[derive(Debug)]
pub struct Renderer {
    render_target: RenderTarget,

    pipes: Pipes,

    gpu: Arc<GpuContext>,
}

impl Renderer {
    pub fn new(gpu: Arc<GpuContext>, width: u32, height: u32) -> Self {
        let render_target_spec = RenderTargetSpecification::default()
            .with_size(width, height)
            .with_label("render target")
            .with_format(wgpu::TextureFormat::Bgra8UnormSrgb);

        let render_target = RenderTarget::new(&gpu, &render_target_spec);

        Self {
            gpu,
            render_target,

            pipes: Pipes::default(),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.render_target.resize(&self.gpu, width, height);
    }

    pub fn destroy(&mut self) {
        self.pipes.destroy();
    }

    pub fn render(&mut self) {
        todo!()
    }

    pub fn render_to_texture(&mut self, color: wgpu::Color, destination_texture: &wgpu::Texture) {
        let gpu = &self.gpu;

        let mut encoder = gpu.create_command_encoder(Some("my encoder"));

        {
            encoder.begin_render_pass(
                &(wgpu::RenderPassDescriptor {
                    label: Some("RenderTarget Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: self.render_target.texture_view(),
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(color),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                }),
            );

            self.render_target
                .copy_to_texture(&mut encoder, destination_texture);
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));

        log::info!("Render Complete!");
    }
}
