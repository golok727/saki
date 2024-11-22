use std::sync::Arc;

pub mod render_target;

use render_target::{RenderTarget, RenderTargetSpecification};

#[derive(Debug)]
pub struct SimplePipe {
    pub pipeline: wgpu::RenderPipeline,
    pub shader: wgpu::ShaderModule,
}

impl SimplePipe {
    pub fn new(gpu: &GpuContext) -> Self {
        let shader =
            gpu.create_shader_labeled(include_str!("./scene/shader.wgsl"), "Simple Shader");

        let layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("thing desciptor"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Simple pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            });

        Self { pipeline, shader }
    }
}

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
    simple_pipe: SimplePipe,

    pub(crate) gpu: Arc<GpuContext>,
}

impl Renderer {
    pub fn new(gpu: Arc<GpuContext>, width: u32, height: u32) -> Self {
        let render_target_spec = RenderTargetSpecification::default()
            .with_size(width, height)
            .with_label("render target")
            .with_format(wgpu::TextureFormat::Bgra8UnormSrgb);

        let render_target = RenderTarget::new(&gpu, &render_target_spec);

        let simple_pipe = SimplePipe::new(&gpu);

        Self {
            gpu,
            render_target,
            simple_pipe,
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
            let mut pass = encoder.begin_render_pass(
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
            pass.set_pipeline(&self.simple_pipe.pipeline);
            pass.draw(0..3, 0..1);
        }

        self.render_target
            .copy_to_texture(&mut encoder, destination_texture);

        gpu.queue.submit(std::iter::once(encoder.finish()));

        log::trace!("Render Complete!");
    }
}
