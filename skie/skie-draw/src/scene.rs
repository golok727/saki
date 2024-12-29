use crate::{
    paint::atlas::{AtlasTextureId, AtlasTextureInfoMap},
    paint::{DrawVert, Mesh, PrimitiveKind},
    traits::IsZero,
    DrawList, Primitive, TextureId,
};

use ahash::HashSet;

#[derive(Debug, Default, Clone)]
pub struct Scene {
    pub(crate) items: Vec<Primitive>,
}

impl Scene {
    pub fn push_layer(&mut self) {
        todo!()
    }

    pub fn pop_layer(&mut self) {
        todo!()
    }

    pub fn add(&mut self, prim: impl Into<Primitive>) {
        self.items.push(prim.into())
    }

    pub fn extend(&mut self, other: &Self) {
        self.items.extend_from_slice(&other.items)
    }

    pub fn clear(&mut self) -> Vec<Primitive> {
        let old: Vec<Primitive> = std::mem::take(&mut self.items);
        old
    }

    pub fn get_required_textures(&self) -> impl Iterator<Item = TextureId> + '_ {
        self.items
            .iter()
            .map(|f| f.texture)
            .collect::<HashSet<_>>()
            .into_iter()
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

            if !prim.can_render() {
                continue;
            }

            let tile_tex_id = entry.texture_id;
            let is_default_texture = tile_tex_id == TextureId::WHITE_TEXTURE;

            let info = self.tex_info.get(&tile_tex_id);

            let uv_middleware = move |mut vertex: DrawVert| {
                // should be Some unless the WHITE_TEX_ID is not inserted by the renderer for some reason
                if let Some(info) = info {
                    if is_default_texture {
                        // should we cache this?
                        vertex.uv = info.uv_to_atlas_space(0.0, 0.0).into();
                    } else {
                        let [u, v] = vertex.uv;
                        vertex.uv = info.uv_to_atlas_space(u, v).into();
                    }
                }

                vertex
            };

            drawlist.set_middleware(uv_middleware);

            match &prim.kind {
                PrimitiveKind::Circle(circle) => {
                    let fill_color = prim.fill.color;

                    drawlist.path.clear();
                    drawlist.path.circle(circle.center, circle.radius);

                    drawlist.fill_path_convex(fill_color, !is_default_texture);
                    if let Some(stroke_style) = &prim.stroke {
                        drawlist.stroke_path(&stroke_style.join())
                    }
                }

                PrimitiveKind::Quad(quad) => {
                    let fill_color = prim.fill.color;

                    if quad.corners.is_zero() && prim.stroke.is_none() {
                        drawlist.add_prim_quad(&quad.bounds, fill_color);
                    } else {
                        drawlist.path.clear();
                        drawlist.path.round_rect(&quad.bounds, &quad.corners);
                        drawlist.fill_path_convex(fill_color, !is_default_texture);

                        if let Some(stroke_style) = &prim.stroke {
                            drawlist.stroke_path(&stroke_style.join())
                        }
                    }
                }

                PrimitiveKind::Path(path) => {
                    drawlist.fill_with_path(path, prim.fill.color);

                    if let Some(stroke_style) = &prim.stroke {
                        let stroke_style = if path.closed {
                            stroke_style.join()
                        } else {
                            *stroke_style
                        };
                        drawlist.stroke_with_path(path, &stroke_style);
                    }
                }
            }
        }

        self.cur_group += 1;

        let mut mesh = drawlist.build();
        mesh.texture = TextureId::Atlas(atlas_tex_id);
        Some(mesh)
    }
}

impl<'a> Iterator for SceneBatchIterator<'a> {
    type Item = Mesh;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_batch()
    }
}
