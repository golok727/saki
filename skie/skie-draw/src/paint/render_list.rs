use crate::{
    paint::{AtlasKey, AtlasTextureInfo, AtlasTextureInfoMap, Mesh, Primitive},
    Brush, DrawList, TextureId,
};

use ahash::HashSet;

use super::Color;

#[derive(Debug, Clone)]
pub(crate) struct GraphicsInstruction {
    primitive: Primitive,
    brush: Brush,
    texture_id: TextureId,
}

impl GraphicsInstruction {
    fn nothing_to_draw(&self) -> bool {
        self.brush.noting_to_draw()
    }

    pub fn textured(primitive: impl Into<Primitive>, texture_id: TextureId) -> Self {
        Self {
            primitive: primitive.into(),
            texture_id,
            brush: Brush::filled(Color::WHITE),
        }
    }

    pub fn brush(primitive: impl Into<Primitive>, brush: Brush) -> Self {
        Self {
            primitive: primitive.into(),
            texture_id: TextureId::WHITE_TEXTURE,
            brush,
        }
    }

    pub fn textured_brush(
        primitive: impl Into<Primitive>,
        texture_id: TextureId,
        brush: Brush,
    ) -> Self {
        Self {
            primitive: primitive.into(),
            texture_id,
            brush,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct RenderList {
    pub(crate) instructions: Vec<GraphicsInstruction>,
}

impl RenderList {
    pub fn add(&mut self, instruction: GraphicsInstruction) {
        self.instructions.push(instruction)
    }

    pub fn clear(&mut self) -> Vec<GraphicsInstruction> {
        let old: Vec<GraphicsInstruction> = std::mem::take(&mut self.instructions);
        old
    }

    pub fn get_required_textures(&self) -> impl Iterator<Item = TextureId> + '_ {
        self.instructions
            .iter()
            .map(|instruction| instruction.texture_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
    }

    pub fn batches(
        &self,
        tex_info: GraphicsTextureInfoMap,
        antialias: bool,
    ) -> impl Iterator<Item = Mesh> + '_ {
        InstructionBatchIterator::new(self, tex_info, antialias)
    }
}

#[derive(Debug)]
struct GroupEntry {
    index: usize,
    texture_id: TextureId,
}

pub type GraphicsTextureInfoMap = AtlasTextureInfoMap<AtlasKey>;

// A simple batcher for now in future we will expand this.
struct InstructionBatchIterator<'a> {
    scene: &'a RenderList,
    groups: Vec<(TextureId, Vec<GroupEntry>)>,
    tex_info: GraphicsTextureInfoMap,
    cur_group: usize,
    antialias: bool,
}

impl<'a> InstructionBatchIterator<'a> {
    pub fn new(scene: &'a RenderList, tex_info: GraphicsTextureInfoMap, antialias: bool) -> Self {
        let mut tex_to_item_idx: ahash::AHashMap<TextureId, Vec<GroupEntry>> = Default::default();

        for (i, instruction) in scene.instructions.iter().enumerate() {
            let render_texture = match &instruction.texture_id {
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
                        texture_id: instruction.texture_id.clone(),
                    });
            }
        }

        let mut groups: Vec<(TextureId, Vec<GroupEntry>)> = tex_to_item_idx.into_iter().collect();

        // FIXME
        groups.sort_by_key(|(_, val)| val.first().map(|v| v.index).unwrap_or(0));

        Self {
            scene,
            tex_info,
            cur_group: 0,
            groups,
            antialias,
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
        drawlist.antialias(self.antialias);

        for entry in &group.1 {
            let instruction = &self.scene.instructions[entry.index];
            let primitive = &instruction.primitive;
            let brush = &instruction.brush;

            if instruction.nothing_to_draw() {
                continue;
            }

            let tex_id = entry.texture_id.clone();
            let is_white_texture = tex_id == TextureId::WHITE_TEXTURE;

            let info: Option<&AtlasTextureInfo> = if let TextureId::AtlasKey(key) = &tex_id {
                self.tex_info.get(key)
            } else {
                None
            };

            let build = |drawlist: &mut DrawList| match &primitive {
                Primitive::Circle(circle) => {
                    let fill_color = brush.fill_style.color;

                    drawlist.path.clear();
                    drawlist.path.circle(circle.center, circle.radius);

                    drawlist.fill_path_convex(fill_color, !is_white_texture);
                    drawlist.stroke_path(&brush.stroke_style.join())
                }

                Primitive::Quad(quad) => {
                    let fill_color = brush.fill_style.color;

                    drawlist.path.clear();
                    drawlist.path.round_rect(&quad.bounds, &quad.corners);
                    drawlist.fill_path_convex(fill_color, !is_white_texture);
                    drawlist.stroke_path(&brush.stroke_style.join())
                }

                Primitive::Path(path) => {
                    // TODO:
                    // drawlist.fill_with_path(path, prim.fill.color);

                    let stroke_style = if path.closed {
                        brush.stroke_style.join()
                    } else {
                        brush.stroke_style
                    };

                    drawlist.stroke_with_path(path, &stroke_style);
                }
            };

            if let Some(info) = info {
                // Convert to atlas space if the texture belongs to the atlas
                drawlist.capture(build).map(|vertex| {
                    if is_white_texture {
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

impl<'a> Iterator for InstructionBatchIterator<'a> {
    type Item = Mesh;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_batch()
    }
}
