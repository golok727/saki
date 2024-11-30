use crate::gpu::GpuContext;
use crate::math::{DevicePixels, Rect, Size};

use parking_lot::Mutex;
use std::sync::Arc;

use super::{TextureFormat, TextureKind, WgpuTexture, WgpuTextureView};

#[derive(Debug, Default)]
struct AtlasTextureList<T: std::fmt::Debug> {
    slots: Vec<T>,
    free_slots: Vec<usize>,
}

impl<T: std::fmt::Debug> AtlasTextureList<T> {
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.slots.iter_mut()
    }
}

impl<T: std::fmt::Debug> std::ops::Index<usize> for AtlasTextureList<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.slots[index]
    }
}

impl<T: std::fmt::Debug> std::ops::IndexMut<usize> for AtlasTextureList<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.slots[index]
    }
}

#[derive(Debug, Clone)]
pub struct AtlasSystem(Arc<Mutex<AtlasSystemState>>);

unsafe impl Send for AtlasSystem {}

#[derive(Debug)]
struct AtlasSystemState {
    gpu: Arc<GpuContext>,
    gray_textures: AtlasTextureList<Option<AtlasTexture>>,
    color_textures: AtlasTextureList<Option<AtlasTexture>>,
}

impl AtlasSystem {
    pub fn new(gpu: Arc<GpuContext>) -> Self {
        Self(Arc::new(Mutex::new(AtlasSystemState {
            gpu,
            gray_textures: Default::default(),
            color_textures: Default::default(),
        })))
    }

    pub fn get_or_insert() {}

    /// Allocates a tile of given size on an available texture slot and returns the tile
    /// use the `upload_texture` method to upload data into tile
    pub fn create_texture(&self, size: Size<DevicePixels>, kind: TextureKind) -> AtlasTile {
        let mut lock = self.0.lock();
        let storage = lock.get_storage_write(&kind);

        if let Some(tile) = storage
            .iter_mut()
            .flatten()
            .rev()
            .find_map(|texture| texture.allocate(size))
        {
            return tile;
        }

        let texture = lock.new_texture(size, kind);
        texture.allocate(size).expect("Error allocating texture!")
    }

    /// Combination of `create_texture` and `upload_texture`
    pub fn create_texture_init(
        &self,
        size: Size<DevicePixels>,
        kind: TextureKind,
        data: &[u8],
    ) -> AtlasTile {
        let tile = self.create_texture(size, kind);
        self.upload_texture(&tile, data);
        tile
    }

    /// Uploads data for the given tile
    pub fn upload_texture(&self, tile: &AtlasTile, data: &[u8]) {
        let lock = self.0.lock();
        let storage = lock.get_storage_read(&tile.texture.kind);
        let texture = storage[tile.texture.slot].as_ref();

        if let Some(texture) = texture {
            lock.gpu.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture.raw,
                    aspect: wgpu::TextureAspect::All,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: tile.bounds.x.into(),
                        y: tile.bounds.y.into(),
                        z: 0,
                    },
                },
                data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(
                        texture.kind.bytes_per_pixel() * u32::from(texture.size.width),
                    ),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: texture.width().into(),
                    height: texture.height().into(),
                    depth_or_array_layers: 1,
                },
            );
        } else {
            log::error!("TEX_NOT_FOUND: Texture upload failed");
        }
    }
}

impl AtlasSystemState {
    #[inline]
    fn get_storage_write(
        &mut self,
        kind: &TextureKind,
    ) -> &mut AtlasTextureList<Option<AtlasTexture>> {
        match kind {
            TextureKind::Grayscale => &mut self.gray_textures,
            TextureKind::Color => &mut self.color_textures,
        }
    }

    #[inline]
    fn get_storage_read(&self, kind: &TextureKind) -> &AtlasTextureList<Option<AtlasTexture>> {
        match kind {
            TextureKind::Grayscale => &self.gray_textures,
            TextureKind::Color => &self.color_textures,
        }
    }

    fn new_texture(&mut self, size: Size<DevicePixels>, kind: TextureKind) -> &mut AtlasTexture {
        const DEFAULT_SIZE: Size<DevicePixels> = Size {
            width: DevicePixels(1024),
            height: DevicePixels(1024),
        };

        let size = DEFAULT_SIZE.max(&size);
        let format = kind.get_texture_format();

        let raw = self.gpu.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas_texture"),
            size: wgpu::Extent3d {
                width: size.width.into(),
                height: size.height.into(),
                depth_or_array_layers: 1,
            },
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            mip_level_count: 1,
            sample_count: 1,
            view_formats: &[],
        });

        let bytes_per_pixel = kind.bytes_per_pixel();
        let width = raw.width();
        let height = raw.height();

        let n_bytes = width * height * bytes_per_pixel;

        let init_data = vec![0u8; n_bytes as usize];

        self.gpu.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &raw,
                aspect: wgpu::TextureAspect::All,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &init_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width * bytes_per_pixel),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = raw.create_view(&wgpu::TextureViewDescriptor::default());
        let allocator = etagere::BucketedAtlasAllocator::new(size.into());

        let storage = self.get_storage_write(&kind);
        let slot = storage.free_slots.pop();

        let atlas_tex = AtlasTexture {
            id: AtlasTextureId {
                kind,
                slot: slot.unwrap_or(storage.slots.len()),
            },
            allocator,
            kind,
            raw,
            view,
            format,
            size,
        };
        if let Some(slot) = slot {
            storage[slot] = Some(atlas_tex);
            storage.slots.get_mut(slot).unwrap().as_mut().unwrap()
        } else {
            storage.slots.push(Some(atlas_tex));
            storage.slots.last_mut().unwrap().as_mut().unwrap()
        }
    }
}

pub struct AtlasTexture {
    id: AtlasTextureId,
    allocator: etagere::BucketedAtlasAllocator,
    raw: wgpu::Texture,
    view: wgpu::TextureView,
    kind: TextureKind,
    format: TextureFormat,
    size: Size<DevicePixels>,
}

impl AtlasTexture {
    fn allocate(&mut self, size: Size<DevicePixels>) -> Option<AtlasTile> {
        let allocation = self.allocator.allocate(size.into())?;
        let id = allocation.id;

        let alloc_rect = allocation.rectangle;

        let bounds: Rect<DevicePixels> = Rect {
            x: (alloc_rect.min.x).into(),
            y: (alloc_rect.min.y).into(),
            width: size.width,
            height: size.height,
        };

        Some(AtlasTile {
            id: id.into(),
            texture: self.id,
            bounds,
        })
    }

    #[inline]
    pub fn kind(&self) -> TextureKind {
        self.kind
    }

    #[inline]
    pub fn format(&self) -> TextureFormat {
        self.format
    }

    #[inline]
    pub fn size(&self) -> Size<DevicePixels> {
        self.size
    }

    #[inline]
    pub fn width(&self) -> DevicePixels {
        self.size.width
    }

    #[inline]
    pub fn height(&self) -> DevicePixels {
        self.size.height
    }

    #[inline]
    pub fn raw(&self) -> &WgpuTexture {
        &self.raw
    }

    #[inline]
    pub fn view(&self) -> &WgpuTextureView {
        &self.view
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct AtlasTextureId {
    kind: TextureKind,
    slot: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct AtlasTileId(u32);

#[derive(Debug, Clone, PartialEq)]
pub struct AtlasTile {
    id: AtlasTileId,
    texture: AtlasTextureId,
    bounds: Rect<DevicePixels>,
}

pub fn create_atlas_system(gpu: Arc<GpuContext>) -> Arc<AtlasSystem> {
    Arc::new(AtlasSystem::new(gpu))
}

impl std::fmt::Debug for AtlasTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Atlas")
            .field(
                "allocator",
                &format!("space = {}", self.allocator.allocated_space()),
            )
            .finish()
    }
}

impl From<etagere::Size> for Size<DevicePixels> {
    fn from(value: etagere::Size) -> Self {
        Self {
            width: value.width.into(),
            height: value.height.into(),
        }
    }
}

impl From<Size<DevicePixels>> for etagere::Size {
    fn from(value: Size<DevicePixels>) -> Self {
        etagere::size2(value.width.into(), value.height.into())
    }
}

impl From<etagere::AllocId> for AtlasTileId {
    fn from(value: etagere::AllocId) -> Self {
        Self(value.serialize())
    }
}

impl From<AtlasTileId> for etagere::AllocId {
    fn from(value: AtlasTileId) -> Self {
        etagere::AllocId::deserialize(value.0)
    }
}
