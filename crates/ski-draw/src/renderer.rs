use std::sync::Arc;

pub mod render_target;

use bytemuck::{Pod, Zeroable};
use render_target::{RenderTarget, RenderTargetSpecification};
use wgpu::util::DeviceExt;

#[derive(Debug)]
pub struct SimplePipe {
    pub pipeline: wgpu::RenderPipeline,
    pub shader: wgpu::ShaderModule,
}

impl SimplePipe {
    pub fn new(gpu: &GpuContext, bind_group_layouts: &[&wgpu::BindGroupLayout]) -> Self {
        let shader =
            gpu.create_shader_labeled(include_str!("./scene/shader.wgsl"), "Simple Shader");

        let layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("thing desciptor"),
                bind_group_layouts,
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
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
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

use crate::{gpu::GpuContext, math::Mat3};

#[derive(Debug, Default)]
pub struct Pipes {
    pub quad: Option<wgpu::RenderPipeline>,
}

impl Pipes {
    pub fn destroy(&mut self) {}
}

#[derive(Debug, Clone, Copy, Zeroable, Pod, Default)]
#[repr(C)]
pub struct GlobalUniformData {
    pub color: [f32; 4],
    pub proj: Mat3,
    _pad: [f32; 3],
}

#[derive(Debug)]
pub struct GlobalUniformsBuffer {
    pub data: GlobalUniformData,
    pub gpu_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bing_group_layout: wgpu::BindGroupLayout,
}

impl GlobalUniformsBuffer {
    pub fn new(gpu: &GpuContext, data: GlobalUniformData) -> Self {
        let gpu_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Global uniform buffer"),
                contents: bytemuck::cast_slice(&[data]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_SRC,
            });

        let layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Global uniform bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Global uniform bind group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: gpu_buffer.as_entire_binding(),
            }],
        });

        Self {
            data,
            gpu_buffer,
            bind_group,
            bing_group_layout: layout,
        }
    }
}

#[derive(Debug)]
pub struct Renderer {
    render_target: RenderTarget,

    pipes: Pipes,

    // mock
    global_uniforms: GlobalUniformsBuffer,
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

        let aspect: f32 = width as f32 / height as f32;

        // let proj = Mat3::ortho(1.0, -1.0 * aspect, 1.0, 1.0 * aspect);
        let proj = Mat3::new();

        let data = GlobalUniformData {
            color: [0.2, 0.4, 0.6, 1.0],
            proj,
            ..Default::default()
        };

        let g_uniform_buffer = GlobalUniformsBuffer::new(&gpu, data);

        let simple_pipe = SimplePipe::new(&gpu, &[&g_uniform_buffer.bing_group_layout]);

        Self {
            gpu,
            render_target,
            simple_pipe,
            global_uniforms: g_uniform_buffer,
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
        let view = destination_texture.create_view(&wgpu::TextureViewDescriptor::default());

        {
            let mut pass = encoder.begin_render_pass(
                &(wgpu::RenderPassDescriptor {
                    label: Some("RenderTarget Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
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
            pass.set_bind_group(0, &self.global_uniforms.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));

        log::trace!("Render Complete!");
    }
}
