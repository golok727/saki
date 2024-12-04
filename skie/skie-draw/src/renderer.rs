use std::sync::Arc;
use std::{cell::Cell, num::NonZeroU64, ops::Range};

use crate::gpu::error::GpuSurfaceCreateError;
use crate::gpu::surface::{GpuSurface, GpuSurfaceSpecification};
use crate::math::Size;
use crate::paint::atlas::AtlasManager;
use crate::paint::{TextureKind, WgpuTextureView};
use crate::{
    gpu::GpuContext,
    math::Mat3,
    paint::{Mesh, TextureId, Vertex, WHITE_TEX_ID},
};

pub mod render_target;

use render_target::{OffscreenRenderTarget, OffscreenRenderTargetSpec};
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

#[derive(Debug)]
struct RendererState {
    gpu: Arc<GpuContext>,

    //FIXME this renderer doesn't need to know about the texture source?
    texture_system: AtlasManager,

    clear_color: wgpu::Color,

    global_uniforms: GlobalUniformsBuffer,

    textures: ahash::AHashMap<TextureId, RendererTexture>,

    scene_pipe: ScenePipe,

    texture_bindgroup_layout: wgpu::BindGroupLayout,
}

impl RendererState {
    #[inline]
    fn sync_global_uniforms(&self) {
        self.global_uniforms.sync(&self.gpu);
    }

    #[inline]
    fn insert_texture(&mut self, tex_id: TextureId, val: RendererTexture) {
        self.textures.insert(tex_id, val);
    }
}

// TODO more surface configuration
#[derive(Debug, Clone)]
pub struct WgpuRendererSpecs {
    pub width: u32,
    pub height: u32,
}

pub trait RenderTarget: std::fmt::Debug {
    fn pre_render(&mut self, gpu: &GpuContext);
    fn begin_frame(&mut self) -> &WgpuTextureView;
    fn end_frame(&mut self);
    fn resize(&mut self, width: u32, height: u32);
}

#[derive(Debug)]
pub struct WgpuRenderer {
    // TODO dyn ?
    render_target: DefaultRenderTarget,
    state: RendererState,
}

impl WgpuRenderer {
    /// Creates a new Windowed renderer
    pub fn windowed(
        gpu: Arc<GpuContext>,
        texture_system: AtlasManager,
        screen: impl Into<wgpu::SurfaceTarget<'static>>,
        specs: &WgpuRendererSpecs,
    ) -> Result<Self, GpuSurfaceCreateError> {
        let width = specs.width;
        let height = specs.height;
        let surface = gpu.create_surface(screen, &(GpuSurfaceSpecification { width, height }))?;

        let render_target = DefaultRenderTarget::Windowed {
            surface,
            current_texture: None,
            current_texture_view: None,
        };

        let state = Self::create_state(gpu, texture_system, specs);

        let mut renderer = Self {
            render_target,
            state,
        };

        renderer.set_atlas_texture(&WHITE_TEX_ID);

        Ok(renderer)
    }
    /// Creates a new offscreen renderer
    pub fn offscreen(
        gpu: Arc<GpuContext>,
        texture_system: AtlasManager,
        specs: &WgpuRendererSpecs,
    ) -> Self {
        let width = specs.width;
        let height = specs.height;

        let render_target_spec = OffscreenRenderTargetSpec::default()
            .with_size(width, height)
            .with_label("render target")
            .with_format(wgpu::TextureFormat::Rgba8UnormSrgb);

        let render_target = OffscreenRenderTarget::new(&gpu, &render_target_spec);

        let render_target = DefaultRenderTarget::Offscreen { render_target };

        let state = Self::create_state(gpu, texture_system, specs);
        let mut renderer = Self {
            render_target,
            state,
        };

        renderer.set_atlas_texture(&WHITE_TEX_ID);

        renderer
    }

    pub fn texture_system(&self) -> &AtlasManager {
        &self.state.texture_system
    }

    pub fn get_capture(&mut self) {
        match &self.render_target {
            DefaultRenderTarget::Offscreen { .. } => todo!(),
            DefaultRenderTarget::Windowed { .. } => {
                panic!("WgpuRenderer::get_capture is only availabkle in offscreen renderer")
            }
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.render_target.resize(width, height);

        let proj = Mat3::ortho(0.0, 0.0, height as f32, width as f32);
        self.state.global_uniforms.map(|data| {
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
                mag_filter: wgpu::FilterMode::Linear,
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

    //FIXME this renderer doesn't need to know about the texture source?
    pub fn set_atlas_texture(&mut self, texture_id: &TextureId) {
        let contains_texture = self
            .state
            .texture_system
            .with_texture::<Option<(TextureId, wgpu::BindGroup)>>(texture_id, |texture| {
                let atlas_tex_id = TextureId::AtlasTexture(texture.id());
                if self.state.textures.contains_key(&atlas_tex_id) {
                    None
                } else {
                    Some((
                        atlas_tex_id,
                        Self::create_texture_bind_group(
                            &self.state.gpu,
                            &self.state.texture_bindgroup_layout,
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
            self.state
                .insert_texture(atlas_tex_id, RendererTexture { bindgroup });
        } else {
            log::trace!(
                "set_atlas_texture: BindGroup exists for {}. skipping",
                texture_id
            )
        }
    }

    pub fn update_buffers(&mut self, data: &[Mesh]) {
        self.state.sync_global_uniforms();

        self.render_target.pre_render(&self.state.gpu);

        let (vertex_count, index_count): (usize, usize) = data.iter().fold((0, 0), |res, mesh| {
            (res.0 + mesh.vertices.len(), res.1 + mesh.indices.len())
        });

        if vertex_count > 0 {
            let vb = &mut self.state.scene_pipe.vertex_buffer;
            vb.slices.clear();

            let required_vertex_buffer_size =
                (std::mem::size_of::<Vertex>() * vertex_count) as wgpu::BufferAddress;

            if vb.capacity < required_vertex_buffer_size {
                vb.capacity = (vb.capacity * 2).max(required_vertex_buffer_size);
                vb.buffer = self.state.gpu.create_vertex_buffer(vb.capacity);
            }

            let mut staging_vertex = self
                .state
                .gpu
                .queue
                .write_buffer_with(
                    &vb.buffer,
                    0,
                    NonZeroU64::new(required_vertex_buffer_size).unwrap(),
                )
                .expect("Failed to create stating buffer for vertex");

            let mut vertex_offset = 0;
            for mesh in data {
                let size = mesh.vertices.len() * std::mem::size_of::<Vertex>();
                let slice = vertex_offset..size + vertex_offset;
                staging_vertex[slice.clone()].copy_from_slice(bytemuck::cast_slice(&mesh.vertices));
                vb.slices.push(slice);
                vertex_offset += size;
            }
        }

        if index_count > 0 {
            let ib = &mut self.state.scene_pipe.index_buffer;
            ib.slices.clear();

            let required_index_buffer_size =
                (std::mem::size_of::<u32>() * index_count) as wgpu::BufferAddress;

            if ib.capacity < required_index_buffer_size {
                ib.capacity = (ib.capacity * 2).max(required_index_buffer_size);
                ib.buffer = self.state.gpu.create_index_buffer(ib.capacity);
            }

            let mut staging_index = self
                .state
                .gpu
                .queue
                .write_buffer_with(
                    &ib.buffer,
                    0,
                    NonZeroU64::new(required_index_buffer_size).unwrap(),
                )
                .expect("Failed to create staging buffer for");

            let mut index_offset = 0;
            for mesh in data {
                let size = mesh.indices.len() * std::mem::size_of::<u32>();
                let slice = index_offset..size + index_offset;
                staging_index[slice.clone()].copy_from_slice(bytemuck::cast_slice(&mesh.indices));
                ib.slices.push(slice);
                index_offset += size;
            }
        }
    }

    pub fn render(&mut self, batches: &[Mesh]) {
        let mut encoder = self.state.gpu.create_command_encoder(Some("my encoder"));

        let view = self.render_target.begin_frame();

        let mut vb_slices = self.state.scene_pipe.vertex_buffer.slices.iter();
        let mut ib_slices = self.state.scene_pipe.index_buffer.slices.iter();

        {
            let mut pass = encoder.begin_render_pass(
                &(wgpu::RenderPassDescriptor {
                    label: Some("RenderTarget Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(self.state.clear_color),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                }),
            );

            pass.set_pipeline(&self.state.scene_pipe.pipeline);
            pass.set_bind_group(0, &self.state.global_uniforms.bind_group, &[]);

            for mesh in batches {
                let texture = mesh.texture;
                if let Some(RendererTexture { bindgroup, .. }) = self.state.textures.get(&texture) {
                    let vb_slice = vb_slices.next().expect("No next vb_slice");
                    let ib_slice = ib_slices.next().expect("No next ib_slice");

                    pass.set_bind_group(1, bindgroup, &[]);
                    pass.set_vertex_buffer(
                        0,
                        self.state
                            .scene_pipe
                            .vertex_buffer
                            .buffer
                            .slice(vb_slice.start as u64..vb_slice.end as u64),
                    );
                    pass.set_index_buffer(
                        self.state
                            .scene_pipe
                            .index_buffer
                            .buffer
                            .slice(ib_slice.start as u64..ib_slice.end as u64),
                        wgpu::IndexFormat::Uint32,
                    );
                    pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
                } else {
                    let _ = vb_slices.next().expect("No next vb_slice");
                    let _ = ib_slices.next().expect("No next ib_slice");
                    log::error!("Texture: {} not found skipping", texture);
                }
            }
        }

        self.state
            .gpu
            .queue
            .submit(std::iter::once(encoder.finish()));

        self.render_target.end_frame();

        log::trace!("Render Complete!");
    }

    pub fn set_clear_color(&mut self, color: wgpu::Color) {
        self.state.clear_color = color;
    }

    fn create_state(
        gpu: Arc<GpuContext>,
        texture_system: AtlasManager,
        specs: &WgpuRendererSpecs,
    ) -> RendererState {
        // Default white texture for mesh with no texture
        texture_system.get_or_insert(&WHITE_TEX_ID, || {
            (
                TextureKind::Color,
                Size {
                    width: (1).into(),
                    height: (1).into(),
                },
                &[255, 255, 255, 255],
            )
        });

        let proj = Mat3::ortho(0.0, 0.0, specs.height as f32, specs.width as f32);

        let uniform_buffer =
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
            &[&uniform_buffer.bing_group_layout, &texture_bindgroup_layout],
        );

        RendererState {
            gpu,
            scene_pipe,
            clear_color: wgpu::Color::WHITE,
            texture_system,
            textures: ahash::AHashMap::new(),
            global_uniforms: uniform_buffer,
            texture_bindgroup_layout,
        }
    }
}

#[derive(Debug)]
enum DefaultRenderTarget {
    Offscreen {
        render_target: OffscreenRenderTarget,
    },

    Windowed {
        surface: GpuSurface,
        current_texture: Option<wgpu::SurfaceTexture>,
        current_texture_view: Option<wgpu::TextureView>,
    },
}

impl RenderTarget for DefaultRenderTarget {
    fn begin_frame(&mut self) -> &WgpuTextureView {
        match self {
            Self::Offscreen { render_target } => render_target.texture_view(),
            DefaultRenderTarget::Windowed {
                surface,
                current_texture,
                current_texture_view,
            } => {
                let surface_texture = surface.surface.get_current_texture().unwrap();
                let view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                *current_texture_view = Some(view);
                *current_texture = Some(surface_texture); // FIXME error handling

                current_texture_view.as_ref().unwrap()
            }
        }
    }

    fn end_frame(&mut self) {
        match self {
            Self::Offscreen { .. } => {
                // NOOP
            }
            DefaultRenderTarget::Windowed {
                current_texture,
                current_texture_view,
                ..
            } => {
                *current_texture_view = None;
                if let Some(texture) = current_texture.take() {
                    texture.present()
                }
            }
        }
    }

    fn pre_render(&mut self, gpu: &GpuContext) {
        match self {
            DefaultRenderTarget::Offscreen { render_target } => render_target.sync(gpu),
            DefaultRenderTarget::Windowed { surface, .. } => surface.sync(gpu),
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        match self {
            DefaultRenderTarget::Windowed { surface, .. } => surface.resize(width, height),
            DefaultRenderTarget::Offscreen { render_target } => render_target.resize(width, height),
        }
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
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,

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
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
