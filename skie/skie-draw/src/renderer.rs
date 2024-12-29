use std::sync::Arc;
use std::{cell::Cell, num::NonZeroU64, ops::Range};

use crate::gpu::CommandEncoder;
use crate::math::{Rect, Size};
use crate::paint::atlas::AtlasManager;
use crate::paint::WgpuTextureView;
use crate::{
    gpu::GpuContext,
    math::Mat3,
    paint::{DrawVert, Mesh, TextureId},
};

use wgpu::util::DeviceExt;

static INITIAL_VERTEX_BUFFER_SIZE: u64 = (std::mem::size_of::<DrawVert>() * 1024) as u64;
static INITIAL_INDEX_BUFFER_SIZE: u64 = (std::mem::size_of::<u32>() * 1024 * 3) as u64;

#[derive(Debug)]
pub struct Renderable {
    pub clip_rect: Rect<u32>,
    pub mesh: Mesh,
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
    dirty: Cell<bool>,
}

impl GlobalUniformsBuffer {
    pub fn new(gpu: &GpuContext, data: GlobalUniformData) -> Self {
        let gpu_buffer = gpu.device.create_buffer_init(
            &(wgpu::util::BufferInitDescriptor {
                label: Some("Global uniform buffer"),
                contents: bytemuck::cast_slice(&[data]),
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            }),
        );

        let layout = gpu.device.create_bind_group_layout(
            &(wgpu::BindGroupLayoutDescriptor {
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
            }),
        );

        let bind_group = gpu.device.create_bind_group(
            &(wgpu::BindGroupDescriptor {
                label: Some("Global uniform bind group"),
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: gpu_buffer.as_entire_binding(),
                }],
            }),
        );

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
pub struct RendererTexture {
    pub bindgroup: wgpu::BindGroup,
}

// TODO more surface configuration
#[derive(Debug, Clone)]
pub struct WgpuRendererSpecs {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct WgpuRenderer {
    gpu: Arc<GpuContext>,

    size: Size<u32>,

    texture_system: AtlasManager,

    global_uniforms: GlobalUniformsBuffer,

    textures: ahash::AHashMap<TextureId, RendererTexture>,

    scene_pipe: ScenePipe,

    texture_bindgroup_layout: wgpu::BindGroupLayout,
}

impl WgpuRenderer {
    pub fn new(
        gpu: Arc<GpuContext>,
        texture_system: AtlasManager,
        specs: &WgpuRendererSpecs,
    ) -> Self {
        let proj = Mat3::ortho(0.0, 0.0, specs.height as f32, specs.width as f32);

        let global_uniforms =
            GlobalUniformsBuffer::new(&gpu, GlobalUniformData { proj: proj.into() });

        let texture_bindgroup_layout = gpu.device.create_bind_group_layout(
            &(wgpu::BindGroupLayoutDescriptor {
                label: Some("skie wgpu::Renderer texture bindgroup layout"),
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
            }),
        );

        let scene_pipe = ScenePipe::new(
            &gpu,
            &[
                &global_uniforms.bing_group_layout,
                &texture_bindgroup_layout,
            ],
        );

        let mut renderer = Self {
            gpu,
            texture_system,
            global_uniforms,
            textures: Default::default(),
            scene_pipe,
            texture_bindgroup_layout,
            size: Size {
                width: specs.width,
                height: specs.height,
            },
        };

        renderer.set_texture_from_atlas(&TextureId::WHITE_TEXTURE);

        renderer
    }

    pub fn size(&self) -> &Size<u32> {
        &self.size
    }

    pub fn gpu(&self) -> &GpuContext {
        &self.gpu
    }

    pub fn texture_system(&self) -> &AtlasManager {
        &self.texture_system
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let proj = Mat3::ortho(0.0, 0.0, height as f32, width as f32);

        self.size.width = width;
        self.size.height = height;

        self.global_uniforms.map(|data| {
            data.proj = proj.into();
        });
    }

    fn create_texture_bind_group(
        gpu: &GpuContext,
        layout: &wgpu::BindGroupLayout,
        view: &WgpuTextureView,
    ) -> wgpu::BindGroup {
        // TODO allow configuration
        let sampler = gpu.device.create_sampler(
            &(wgpu::SamplerDescriptor {
                label: Some("skie_draw texture sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                lod_max_clamp: Default::default(),
                lod_min_clamp: Default::default(),
                compare: None,
                anisotropy_clamp: 1,
                border_color: None,
            }),
        );

        let bindgroup = gpu.device.create_bind_group(
            &(wgpu::BindGroupDescriptor {
                label: Some("skie_draw texture bind group"),
                layout,
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
            }),
        );

        bindgroup
    }

    pub fn set_texture_from_atlas(&mut self, texture_id: &TextureId) {
        let contains_texture = self
            .texture_system
            .with_texture::<Option<(TextureId, wgpu::BindGroup)>>(texture_id, |texture| {
                let atlas_tex_id = TextureId::Atlas(texture.id());
                if self.textures.contains_key(&atlas_tex_id) {
                    None
                } else {
                    Some((
                        atlas_tex_id,
                        Self::create_texture_bind_group(
                            &self.gpu,
                            &self.texture_bindgroup_layout,
                            texture.view(),
                        ),
                    ))
                }
            });

        if contains_texture.is_none() {
            log::error!(
                "ATLAS_TEXTURE_NOT_FOUND: (set_atlas_texture) {}",
                texture_id
            );
            return;
        }

        let need_to_add = contains_texture.unwrap();

        if let Some((atlas_tex_id, bindgroup)) = need_to_add {
            self.textures
                .insert(atlas_tex_id, RendererTexture { bindgroup });
        } else {
            log::trace!(
                "set_atlas_texture: BindGroup exists for {}. skipping",
                texture_id
            )
        }
    }

    pub fn set_renderables(&mut self, renderables: &[Renderable]) {
        let (vertex_count, index_count): (usize, usize) =
            renderables.iter().fold((0, 0), |res, renderable| {
                (
                    res.0 + renderable.mesh.vertices.len(),
                    res.1 + renderable.mesh.indices.len(),
                )
            });

        if vertex_count > 0 {
            let vb = &mut self.scene_pipe.vertex_buffer;
            vb.slices.clear();

            let required_vertex_buffer_size =
                (std::mem::size_of::<DrawVert>() * vertex_count) as wgpu::BufferAddress;

            if vb.capacity < required_vertex_buffer_size {
                vb.capacity = (vb.capacity * 2).max(required_vertex_buffer_size);
                vb.buffer = self.gpu.create_vertex_buffer(vb.capacity);
            }

            let mut staging_vertex = self
                .gpu
                .queue
                .write_buffer_with(
                    &vb.buffer,
                    0,
                    NonZeroU64::new(required_vertex_buffer_size).unwrap(),
                )
                .expect("Failed to create stating buffer for vertex");

            let mut vertex_offset = 0;

            for renderable in renderables {
                let size = renderable.mesh.vertices.len() * std::mem::size_of::<DrawVert>();
                let slice = vertex_offset..size + vertex_offset;
                staging_vertex[slice.clone()]
                    .copy_from_slice(bytemuck::cast_slice(&renderable.mesh.vertices));
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
                ib.buffer = self.gpu.create_index_buffer(ib.capacity);
            }

            let mut staging_index = self
                .gpu
                .queue
                .write_buffer_with(
                    &ib.buffer,
                    0,
                    NonZeroU64::new(required_index_buffer_size).unwrap(),
                )
                .expect("Failed to create staging buffer for");

            let mut index_offset = 0;
            for renderable in renderables {
                let size = renderable.mesh.indices.len() * std::mem::size_of::<u32>();
                let slice = index_offset..size + index_offset;
                staging_index[slice.clone()]
                    .copy_from_slice(bytemuck::cast_slice(&renderable.mesh.indices));
                ib.slices.push(slice);
                index_offset += size;
            }
        }
    }
    pub fn create_command_encoder(&self) -> CommandEncoder {
        self.gpu
            .create_command_encoder(Some("skie_command_encoder"))
    }

    pub fn render(&mut self, render_pass: &mut wgpu::RenderPass<'_>, renderables: &[Renderable]) {
        self.global_uniforms.sync(&self.gpu);
        let mut vb_slices = self.scene_pipe.vertex_buffer.slices.iter();
        let mut ib_slices = self.scene_pipe.index_buffer.slices.iter();
        render_pass.set_pipeline(&self.scene_pipe.pipeline);

        render_pass.set_bind_group(0, &self.global_uniforms.bind_group, &[]);

        log::info!("Rendering {} renderables", renderables.len());

        for renderable in renderables {
            let scissor = &renderable.clip_rect;
            render_pass.set_scissor_rect(
                scissor.origin.x,
                scissor.origin.y,
                scissor.size.width.min(self.size.width),
                scissor.size.height.min(self.size.height),
            );

            let texture = renderable.mesh.texture;
            if let Some(RendererTexture { bindgroup, .. }) = self.textures.get(&texture) {
                let vb_slice = vb_slices.next().expect("No next vb_slice");
                let ib_slice = ib_slices.next().expect("No next ib_slice");

                render_pass.set_bind_group(1, bindgroup, &[]);
                render_pass.set_vertex_buffer(
                    0,
                    self.scene_pipe
                        .vertex_buffer
                        .buffer
                        .slice(vb_slice.start as u64..vb_slice.end as u64),
                );
                render_pass.set_index_buffer(
                    self.scene_pipe
                        .index_buffer
                        .buffer
                        .slice(ib_slice.start as u64..ib_slice.end as u64),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..renderable.mesh.indices.len() as u32, 0, 0..1);
            } else {
                let _ = vb_slices.next().expect("No next vb_slice");
                let _ = ib_slices.next().expect("No next ib_slice");
                log::error!("Texture: {} not found skipping", texture);
            }
        }

        render_pass.set_scissor_rect(0, 0, self.size.width, self.size.height);
    }

    pub fn end(&mut self) {
        self.scene_pipe.vertex_buffer.slices.clear();
        self.scene_pipe.index_buffer.slices.clear();
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

        let layout = gpu.device.create_pipeline_layout(
            &(wgpu::PipelineLayoutDescriptor {
                label: Some("Scenepipe layout"),
                bind_group_layouts,
                push_constant_ranges: &[],
            }),
        );

        let vbo_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<DrawVert>() as wgpu::BufferAddress,

            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
        };

        let pipeline = gpu.device.create_render_pipeline(
            &(wgpu::RenderPipelineDescriptor {
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
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::OneMinusDstAlpha,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
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
            }),
        );

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
