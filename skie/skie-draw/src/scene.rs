use crate::{
    paint::{
        atlas::{AtlasKeyImpl, AtlasTextureInfo, AtlasTextureInfoMap},
        AtlasKey, Mesh, PrimitiveKind,
    },
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

    pub fn add(&mut self, prim: Primitive) {
        self.items.push(prim)
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
            .map(|f| f.texture.clone())
            .collect::<HashSet<_>>()
            .into_iter()
    }

    pub fn batches(&self, tex_info: SceneTextureInfoMap) -> impl Iterator<Item = Mesh> + '_ {
        SceneBatchIterator::new(self, tex_info)
    }
}

#[derive(Debug)]
struct GroupEntry {
    index: usize,
    texture_id: TextureId,
}

pub type SceneTextureInfoMap = AtlasTextureInfoMap<AtlasKey>;

// A simple batcher for now in future we will expand this.
struct SceneBatchIterator<'a> {
    scene: &'a Scene,
    groups: Vec<(TextureId, Vec<GroupEntry>)>,
    tex_info: SceneTextureInfoMap,
    cur_group: usize,
}

impl<'a> SceneBatchIterator<'a> {
    pub fn new(scene: &'a Scene, tex_info: SceneTextureInfoMap) -> Self {
        let mut tex_to_item_idx: ahash::AHashMap<TextureId, Vec<GroupEntry>> = Default::default();

        for (i, prim) in scene.items.iter().enumerate() {
            let tex = prim.texture.clone();

            let render_texture = match &tex {
                TextureId::AtlasKey(key) => {
                    let info = tex_info.get(key);
                    info.map(|info| TextureId::Atlas(info.tile.texture))
                }
                other => Some(other.clone()),
            };

            if let Some(render_texture) = render_texture {
                tex_to_item_idx
                    .entry(render_texture)
                    .or_default()
                    .push(GroupEntry {
                        index: i,
                        texture_id: tex,
                    });
            }
        }

        let mut groups: Vec<(TextureId, Vec<GroupEntry>)> = tex_to_item_idx.into_iter().collect();

        // FIXME: Not right
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

        // FIXME: no need to build mesh here
        let group = &self.groups[self.cur_group];

        let render_texture = group.0.clone();

        let mut drawlist = DrawList::default();

        for entry in &group.1 {
            let prim = &self.scene.items[entry.index];

            if !prim.can_render() {
                continue;
            }

            let tex_id = entry.texture_id.clone();
            let mut is_default_texture = false;

            let info: Option<&AtlasTextureInfo> = if let TextureId::AtlasKey(key) = &tex_id {
                is_default_texture = matches!(key, &AtlasKey::WHITE_TEXTURE_KEY);
                self.tex_info.get(key)
            } else {
                None
            };

            let build = |drawlist: &mut DrawList| match &prim.kind {
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

                PrimitiveKind::Text(_) => todo!("text is not implemented yet"),
            };

            if let Some(info) = info {
                // Convert to atlas space if the texture belongs to the atlas
                drawlist.capture(build).map(|vertex| {
                    if is_default_texture {
                        vertex.uv = info.uv_to_atlas_space(0.0, 0.0).into();
                    } else {
                        vertex.uv = info.uv_to_atlas_space(vertex.uv[0], vertex.uv[1]).into();
                    }
                });
            } else {
                // Non atlas texture
                build(&mut drawlist)
            }
        }

        self.cur_group += 1;

        let mut mesh = drawlist.build();
        mesh.texture = render_texture;
        Some(mesh)
    }
}

impl<'a> Iterator for SceneBatchIterator<'a> {
    type Item = Mesh;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_batch()
    }
}
