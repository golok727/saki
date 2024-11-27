use std::{cell::Cell, collections::HashMap, num::NonZeroU64, ops::Range};

use crate::{
    gpu::GpuContext,
    math::Mat3,
    paint::{Mesh, TextureId, Vertex},
};

pub mod render_target;

use render_target::{RenderTarget, RenderTargetSpecification};
use wgpu::util::DeviceExt;

static INITIAL_VERTEX_BUFFER_SIZE: u64 = (std::mem::size_of::<Vertex>() * 1024) as u64;
static INITIAL_INDEX_BUFFER_SIZE: u64 = (std::mem::size_of::<u32>() * 1024 * 3) as u64;

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
    dirty: Cell<bool>,
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
            dirty: Cell::new(false),
        }
    }

    pub fn set_data(&mut self, data: GlobalUniformData) {
        self.data = data;
        self.dirty.set(true);
    }

    pub fn map(&mut self, f: impl FnOnce(&mut GlobalUniformData)) {
        f(&mut self.data);
        self.dirty.set(true);
    }

    pub fn sync(&self, gpu: &GpuContext) {
        if !self.dirty.get() {
            return;
        }

        log::trace!("Global uniform buffer sync");

        gpu.queue
            .write_buffer(&self.gpu_buffer, 0, bytemuck::cast_slice(&[self.data]));

        self.dirty.set(false);
    }
}

#[derive(Debug)]
struct RendererTexture {
    raw: Option<wgpu::Texture>,
    id: TextureId,
    bindgroup: wgpu::BindGroup,
}

#[derive(Debug)]
pub struct Renderer {
    // TODO maybe switch renderer to a enum variant Renderer::Windowed Renderer::Default
    render_target: RenderTarget,

    global_uniforms: GlobalUniformsBuffer,

    textures: HashMap<TextureId, RendererTexture>,

    next_texture_id: usize,

    scene_pipe: ScenePipe,

    #[allow(unused)]
    pub(crate) texture_bindgroup_layout: wgpu::BindGroupLayout,
}

impl Renderer {
    pub fn new(gpu: &GpuContext, width: u32, height: u32) -> Self {
        let render_target_spec = RenderTargetSpecification::default()
            .with_size(width, height)
            .with_label("render target")
            .with_format(wgpu::TextureFormat::Rgba8UnormSrgb);

        let render_target = RenderTarget::new(gpu, &render_target_spec);

        let proj = Mat3::ortho(0., 0., height as f32, width as f32);

        let uniform_buffer =
            GlobalUniformsBuffer::new(gpu, GlobalUniformData { proj: proj.into() });

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

        let scene_pipe = ScenePipe::new(gpu, &[&uniform_buffer.bing_group_layout]);

        Self {
            render_target,
            scene_pipe,
            textures: HashMap::new(),
            next_texture_id: 1,
            global_uniforms: uniform_buffer,
            texture_bindgroup_layout,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.render_target.resize(width, height);

        let proj = Mat3::ortho(0., 0., height as f32, width as f32);
        self.global_uniforms.map(|data| {
            data.proj = proj.into();
        });
    }

    pub fn set_native_texture(&mut self, gpu: &GpuContext, view: &wgpu::TextureView) -> TextureId {
        let tick = self.next_texture_id;
        self.next_texture_id += 1;

        self.set_native_texture_impl(gpu, TextureId::User(tick), view)
    }

    pub(crate) fn set_native_texture_impl(
        &mut self,
        gpu: &GpuContext,
        id: TextureId,
        view: &wgpu::TextureView,
        // TODO texture options
    ) -> TextureId {
        // TODO make it configurable
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ski_draw texture sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: Default::default(),
            lod_max_clamp: Default::default(),
            compare: None,
            anisotropy_clamp: Default::default(),
            border_color: None,
        });

        let bindgroup = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ski_draw texture bind group"),
            layout: &self.texture_bindgroup_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.textures.insert(
            id,
            RendererTexture {
                raw: None,
                id,
                bindgroup,
            },
        );

        id
    }

    // call this before rendering
    pub fn update_buffers(&mut self, gpu: &GpuContext, data: &[Mesh]) {
        self.global_uniforms.sync(gpu);
        self.render_target.sync(gpu);

        let (vertex_count, index_count): (usize, usize) = data.iter().fold((0, 0), |res, mesh| {
            (res.0 + mesh.vertices.len(), res.1 + mesh.indices.len())
        });

        if vertex_count > 0 {
            let vb = &mut self.scene_pipe.vertex_buffer;
            vb.slices.clear();

            let required_vertex_buffer_size =
                (std::mem::size_of::<Vertex>() * vertex_count) as wgpu::BufferAddress;

            if vb.capacity < required_vertex_buffer_size {
                vb.capacity = (vb.capacity * 2).max(required_vertex_buffer_size);
                vb.buffer = gpu.create_vertex_buffer(vb.capacity);
            }

            let mut staging_vertex = gpu
                .queue
                .write_buffer_with(
                    &vb.buffer,
                    0,
                    NonZeroU64::new(required_vertex_buffer_size).unwrap(),
                )
                .expect("failed to create staging vertex buffer");

            let mut vertex_offset = 0;
            for mesh in data {
                let size = mesh.vertices.len() * std::mem::size_of::<Vertex>();
                let slice = vertex_offset..(size + vertex_offset);
                staging_vertex[slice.clone()].copy_from_slice(bytemuck::cast_slice(&mesh.vertices));
                vb.slices.push(slice);
                vertex_offset += size;
            }
        }

        if index_count > 0 {
            let ib = &mut self.scene_pipe.index_buffer;
            ib.slices.clear();

            let required_index_buffer_size =
                (std::mem::size_of::<u32>() * index_count) as wgpu::BufferAddress;

            if ib.capacity < required_index_buffer_size {
                ib.capacity = (ib.capacity * 2).max(required_index_buffer_size);
                ib.buffer = gpu.create_index_buffer(ib.capacity);
            }

            let mut staging_index = gpu
                .queue
                .write_buffer_with(
                    &ib.buffer,
                    0,
                    NonZeroU64::new(required_index_buffer_size).unwrap(),
                )
                .expect("failed to create staging vertex buffer");

            let mut index_offset = 0;
            for mesh in data {
                let size = mesh.indices.len() * std::mem::size_of::<u32>();
                let slice = index_offset..(size + index_offset);
                staging_index[slice.clone()].copy_from_slice(bytemuck::cast_slice(&mesh.indices));
                ib.slices.push(slice);
                index_offset += size;
            }
        }
    }

    pub fn render(
        &mut self,
        gpu: &GpuContext,
        clear_color: wgpu::Color,
        batches: &[Mesh],
        destination_texture: &wgpu::Texture,
    ) {
        let mut encoder = gpu.create_command_encoder(Some("my encoder"));
        let view = destination_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut vb_slices = self.scene_pipe.vertex_buffer.slices.iter();
        let mut ib_slices = self.scene_pipe.index_buffer.slices.iter();

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
            pass.set_pipeline(&self.scene_pipe.pipeline);
            pass.set_bind_group(0, &self.global_uniforms.bind_group, &[]);

            for mesh in batches {
                // TODO will be removed after adding texture system
                if mesh.texture.is_some() {
                    let _ = vb_slices.next().expect("No next thing vb_slice");
                    let _ = ib_slices.next().expect("No next thing ib_slice");
                    log::error!("Textures are not supported yet")
                } else {
                    let vb_slice = vb_slices.next().expect("No next thing vb_slice");
                    let ib_slice = ib_slices.next().expect("No next thing ib_slice");

                    // TODO texture bind group
                    pass.set_vertex_buffer(
                        0,
                        self.scene_pipe
                            .vertex_buffer
                            .buffer
                            .slice(vb_slice.start as u64..vb_slice.end as u64),
                    );
                    pass.set_index_buffer(
                        self.scene_pipe
                            .index_buffer
                            .buffer
                            .slice(ib_slice.start as u64..ib_slice.end as u64),
                        wgpu::IndexFormat::Uint32,
                    );
                    pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
                }
            }
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));

        log::trace!("Render Complete!");
    }
}

#[derive(Debug)]
struct BatchBuffer {
    buffer: wgpu::Buffer,
    slices: Vec<Range<usize>>,
    capacity: wgpu::BufferAddress,
}

#[derive(Debug)]
struct ScenePipe {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: BatchBuffer,
    index_buffer: BatchBuffer,
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
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,

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

        let vertex_buffer = BatchBuffer {
            buffer: gpu.create_vertex_buffer(INITIAL_VERTEX_BUFFER_SIZE),
            slices: Vec::with_capacity(64),
            capacity: INITIAL_VERTEX_BUFFER_SIZE,
        };

        let index_buffer = BatchBuffer {
            buffer: gpu.create_index_buffer(INITIAL_INDEX_BUFFER_SIZE),
            slices: Vec::with_capacity(64),
            capacity: INITIAL_INDEX_BUFFER_SIZE,
        };

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
        }
    }
}
