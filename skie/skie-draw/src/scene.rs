use std::cell::Cell;
use std::cell::RefCell;

use crate::paint::atlas::AtlasTextureId;
use crate::paint::atlas::AtlasTextureInfoMap;
use crate::paint::path::GeometryPath;
use crate::paint::DrawList;
use crate::paint::DrawVert;
use crate::paint::Mesh;
use crate::paint::Path2D;
use crate::paint::PathId;
use crate::paint::Primitive;
use crate::paint::PrimitiveKind;
use crate::paint::TextureId;
use crate::paint::DEFAULT_PATH_ID;
use crate::traits::IsZero;

pub(crate) type PathCache = ahash::AHashMap<PathId, GeometryPath>;

#[derive(Debug, Clone)]
pub struct Scene {
    pub(crate) items: Vec<Primitive>,
    pub(crate) path_cache: RefCell<PathCache>,
    pub(crate) next_path_id: Cell<usize>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            next_path_id: Cell::new(1),
            items: Default::default(),
            path_cache: Default::default(),
        }
    }
}

impl Scene {
    pub fn push_layer(&mut self) {
        todo!()
    }

    pub fn pop_layer(&mut self) {
        todo!()
    }

    pub fn create_path(&self) -> Path2D {
        let next_id = self.next_path_id.get();
        self.next_path_id.set(next_id + 1);

        Path2D {
            id: PathId(next_id),
            ..Default::default()
        }
    }

    pub fn add(&mut self, mut prim: Primitive) {
        if let PrimitiveKind::Path(ref mut path) = &mut prim.kind {
            if path.id == DEFAULT_PATH_ID {
                let next_id = self.next_path_id.get();
                self.next_path_id.set(next_id + 1);
                path.id = PathId(next_id);
            }
        }

        self.items.push(prim)
    }

    pub fn clear(&mut self) -> Vec<Primitive> {
        let old: Vec<Primitive> = std::mem::take(&mut self.items);
        old
    }

    pub fn get_required_textures(&self) -> impl Iterator<Item = TextureId> + '_ {
        self.items.iter().map(|f| f.texture)
    }

    pub fn batches(&self, tex_info: AtlasTextureInfoMap) -> impl Iterator<Item = Mesh> + '_ {
        SceneBatchIterator::new(self, tex_info)
    }
}

#[derive(Debug)]
struct GroupEntry {
    index: usize,
    texture_id: TextureId,
}

// A simple batcher for now in future we will expand this.
struct SceneBatchIterator<'a> {
    scene: &'a Scene,
    groups: Vec<(AtlasTextureId, Vec<GroupEntry>)>,
    tex_info: AtlasTextureInfoMap,
    cur_group: usize,
}

impl<'a> SceneBatchIterator<'a> {
    pub fn new(scene: &'a Scene, tex_info: AtlasTextureInfoMap) -> Self {
        let mut tex_to_item_idx: ahash::AHashMap<AtlasTextureId, Vec<GroupEntry>> =
            Default::default();

        for (i, prim) in scene.items.iter().enumerate() {
            let tex = prim.texture;
            let info = tex_info.get(&tex);
            let atlas_tex_id = info.map(|i| i.atlas_texture);

            if let Some(atlas_tex_id) = atlas_tex_id {
                tex_to_item_idx
                    .entry(atlas_tex_id)
                    .or_default()
                    .push(GroupEntry {
                        index: i,
                        texture_id: tex,
                    });
            } else {
                log::error!("Can't find {} in atlas", tex);
                continue;
            }
        }

        let mut groups: Vec<(AtlasTextureId, Vec<GroupEntry>)> =
            tex_to_item_idx.into_iter().collect();

        // FIXME: Is this correct ?
        groups.sort_by_key(|(_, val)| val.first().map(|v| v.index).unwrap_or(0));

        log::trace!("Batches: {}", groups.len());

        Self {
            scene,
            tex_info,
            cur_group: 0,
            groups,
        }
    }

    pub fn next_batch(&mut self) -> Option<Mesh> {
        if self.cur_group >= self.groups.len() {
            return None;
        }

        let group = &self.groups[self.cur_group];

        let atlas_tex_id = group.0;

        let mut drawlist = DrawList::default();

        for entry in &group.1 {
            let prim = &self.scene.items[entry.index];

            let tile_tex_id = entry.texture_id;
            let is_default_texture = tile_tex_id == TextureId::WHITE_TEXTURE;

            let info = self.tex_info.get(&tile_tex_id);

            let uv_middleware = move |mut vertex: DrawVert| {
                // should be Some unless the WHITE_TEX_ID is not inserted by the renderer for some reason
                if let Some(info) = info {
                    if is_default_texture {
                        // should we cache this?
                        vertex.uv = info.uv_to_atlas_space(0.0, 0.0);
                    } else {
                        let [u, v] = vertex.uv;
                        vertex.uv = info.uv_to_atlas_space(u, v);
                    }
                }

                vertex
            };

            drawlist.set_middleware(uv_middleware);

            // we purposefully inline calls here
            match &prim.kind {
                // TODO: ? Should be move this to a trait to build the shape
                PrimitiveKind::Circle(circle) => {
                    let fill_color = prim.fill.color;

                    drawlist.path.clear();
                    drawlist.path.arc(
                        circle.center,
                        circle.radius,
                        0.0,
                        std::f32::consts::TAU,
                        false,
                    );

                    drawlist.fill_path_convex(fill_color);

                    if prim.stroke.is_some() {
                        // TODO: stroke the current_path
                        log::error!("Stroke is not implemented yet")
                    }
                }

                PrimitiveKind::Quad(quad) => {
                    let fill_color = prim.fill.color;

                    if quad.corners.is_zero() {
                        drawlist.add_prim_quad(&quad.bounds, fill_color);
                    } else {
                        drawlist.path.clear();
                        drawlist.path.round_rect(&quad.bounds, &quad.corners);
                        drawlist.fill_path_convex(fill_color);
                    }

                    if prim.stroke.is_some() {
                        // TODO: stroke the current_path
                        log::error!("Stroke is not implemented yet")
                    }
                }

                PrimitiveKind::Path(path) => {
                    self.with_path(path, |path| {
                        // FIXME: use drawlist fill or stroke path after adding earcut
                        drawlist.fill_with_path(path);
                    });
                }
            }
        }

        self.cur_group += 1;

        Some(drawlist.build(TextureId::AtlasTexture(atlas_tex_id)))
    }

    fn with_path<R>(&self, path: &Path2D, mut f: impl FnMut(&GeometryPath) -> R) -> R {
        let mut lock = self.scene.path_cache.borrow_mut();
        let exists = lock.get(&path.id);

        if let (Some(geometry_path), false) = (exists, path.dirty.get()) {
            f(geometry_path)
        } else {
            path.dirty.set(false);
            let geometry_path = GeometryPath::from(path);
            let res = f(&geometry_path);
            lock.insert(path.id, geometry_path);
            res
        }
    }
}

impl<'a> Iterator for SceneBatchIterator<'a> {
    type Item = Mesh;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_batch()
    }
}
