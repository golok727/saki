use std::sync::Arc;

use crate::{
    gpu::GpuContext,
    math::{Mat3, Rect, Vec2},
    scene::{Primitive, Scene},
};

pub mod render_target;

use render_target::{RenderTarget, RenderTargetSpecification};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct SceneVertex {
    position: [f32; 2],
    color: [f32; 4],
}

fn wgpu_color_to_array(color: wgpu::Color) -> [f32; 4] {
    [
        color.r as f32,
        color.g as f32,
        color.b as f32,
        color.a as f32,
    ]
}

impl SceneVertex {
    fn new(pos: Vec2<f32>, color: wgpu::Color) -> Self {
        Self {
            position: pos.into(),
            color: wgpu_color_to_array(color),
        }
    }
}

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
            attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
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

    pub fn use_scene<F>(&mut self, gpu: &GpuContext, scene: &Scene, f: F)
    where
        F: FnOnce(&wgpu::RenderPipeline, &wgpu::Buffer, &wgpu::Buffer, u32),
    {
        let mut vertices: Vec<SceneVertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        let mut index_offset: u16 = 0;

        for prim in &scene.prims {
            match prim {
                Primitive::Quad(quad) => {
                    let Rect {
                        x,
                        y,
                        width,
                        height,
                    } = quad.bounds;

                    let color = quad.background_color;
                    vertices.push(SceneVertex::new((x, y).into(), color)); // Top-left
                    vertices.push(SceneVertex::new((x + width, y).into(), color)); // Top-right
                    vertices.push(SceneVertex::new((x, y + height).into(), color)); // Bottom-left
                    vertices.push(SceneVertex::new((x + width, y + height).into(), color)); // Bottom-right

                    indices.extend_from_slice(&[
                        index_offset,
                        index_offset + 1,
                        index_offset + 2,
                        index_offset + 2,
                        index_offset + 1,
                        index_offset + 3,
                    ]);

                    index_offset += 4;
                }
            }
        }

        let vbo = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Scene pipe vbo"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            });

        let ibo = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Scene pipe ibo"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            });

        f(&self.pipeline, &vbo, &ibo, indices.len() as u32);
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

        let scene_pipe = ScenePipe::new(&gpu, &[&uniform_buffer.bing_group_layout]);

        Self {
            gpu,
            render_target,
            scene_pipe,
            global_uniforms: uniform_buffer,
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
            .use_scene(gpu, scene, |pipeline, vbo, ibo, indices| {
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
                    pass.set_index_buffer(ibo.slice(..), wgpu::IndexFormat::Uint16);
                    pass.set_bind_group(0, &self.global_uniforms.bind_group, &[]);
                    pass.draw_indexed(0..indices, 0, 0..1);
                }
                gpu.queue.submit(std::iter::once(encoder.finish()));
            });

        log::trace!("Render Complete!");
    }
}
