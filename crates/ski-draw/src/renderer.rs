use std::sync::Arc;

use crate::{
    gpu::GpuContext,
    math::Mat3,
    paint::{DrawList, PrimitiveKind, SceneVertex},
    scene::Scene,
};

pub mod render_target;

use render_target::{RenderTarget, RenderTargetSpecification};
use wgpu::util::DeviceExt;

#[derive(Debug)]
pub struct ScenePipe {
    pub pipeline: wgpu::RenderPipeline,
    pub shader: wgpu::ShaderModule,
}

impl ScenePipe {
    pub fn new(gpu: &GpuContext, bind_group_layouts: &[&wgpu::BindGroupLayout]) -> Self {
        let shader = gpu.create_shader_labeled(include_str!("./scene/shader.wgsl"), "Scene Shader");

        let layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Scenepipe layout"),
                bind_group_layouts,
                push_constant_ranges: &[],
            });

        let vbo_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SceneVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
        };

        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Scene pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    buffers: &[vbo_layout],
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
                    front_face: wgpu::FrontFace::default(),
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

    pub fn with_scene<F>(&mut self, gpu: &GpuContext, scene: &Scene, f: F)
    where
        F: FnOnce(&wgpu::RenderPipeline, &wgpu::Buffer, &wgpu::Buffer, u32),
    {
        let mut drawlist = DrawList::default();

        for prim in &scene.items {
            match &prim.kind {
                PrimitiveKind::Quad(quad) => {
                    drawlist.push_quad(quad, prim.texture.is_some());
                }
            }
        }

        let vbo = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Scene pipe vbo"),
                contents: bytemuck::cast_slice(&drawlist.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let ibo = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Scene pipe ibo"),
                contents: bytemuck::cast_slice(&drawlist.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        f(&self.pipeline, &vbo, &ibo, drawlist.indices.len() as u32);
    }

    pub fn dispose(&mut self) {}
}

#[derive(Default, Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct GlobalUniformData {
    proj: [[f32; 4]; 4],
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
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            });

        let layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Global uniform bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
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

    pub fn sync(&self, gpu: &GpuContext) {
        gpu.queue
            .write_buffer(&self.gpu_buffer, 0, bytemuck::cast_slice(&[self.data]))
    }
}

#[derive(Debug)]
pub struct Renderer {
    render_target: RenderTarget,

    global_uniforms: GlobalUniformsBuffer,

    scene_pipe: ScenePipe,

    pub(crate) gpu: Arc<GpuContext>,

    #[allow(unused)]
    pub(crate) texture_bindgroup_layout: wgpu::BindGroupLayout,
}

impl Renderer {
    pub fn new(gpu: Arc<GpuContext>, width: u32, height: u32) -> Self {
        let render_target_spec = RenderTargetSpecification::default()
            .with_size(width, height)
            .with_label("render target")
            .with_format(wgpu::TextureFormat::Bgra8UnormSrgb);

        let render_target = RenderTarget::new(&gpu, &render_target_spec);

        let proj = Mat3::ortho(0., 0., height as f32, width as f32);

        let uniform_buffer =
            GlobalUniformsBuffer::new(&gpu, GlobalUniformData { proj: proj.into() });

        let texture_bindgroup_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Ski wgpu::Renderer texture bindgroup layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let scene_pipe = ScenePipe::new(&gpu, &[&uniform_buffer.bing_group_layout]);

        Self {
            gpu,
            render_target,
            scene_pipe,
            global_uniforms: uniform_buffer,
            texture_bindgroup_layout,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.render_target.resize(&self.gpu, width, height);

        let proj = Mat3::ortho(0., 0., height as f32, width as f32);
        self.global_uniforms.data.proj = proj.into();
        self.global_uniforms.sync(&self.gpu);
    }

    pub fn destroy(&mut self) {
        // todo
    }

    pub fn render(&mut self) {
        todo!()
    }

    pub fn render_to_texture(
        &mut self,
        clear_color: wgpu::Color,
        scene: &Scene,
        destination_texture: &wgpu::Texture,
    ) {
        let gpu = &self.gpu;

        let mut encoder = gpu.create_command_encoder(Some("my encoder"));
        let view = destination_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.scene_pipe
            .with_scene(gpu, scene, |pipeline, vbo, ibo, indices| {
                {
                    let mut pass = encoder.begin_render_pass(
                        &(wgpu::RenderPassDescriptor {
                            label: Some("RenderTarget Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(clear_color),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            occlusion_query_set: None,
                            timestamp_writes: None,
                        }),
                    );
                    pass.set_pipeline(pipeline);
                    pass.set_vertex_buffer(0, vbo.slice(..));
                    pass.set_index_buffer(ibo.slice(..), wgpu::IndexFormat::Uint32);
                    pass.set_bind_group(0, &self.global_uniforms.bind_group, &[]);
                    // pass.set_bind_group(1, None, &[]);
                    pass.draw_indexed(0..indices, 0, 0..1);
                }

                gpu.queue.submit(std::iter::once(encoder.finish()));
            });

        log::trace!("Render Complete!");
    }
}
