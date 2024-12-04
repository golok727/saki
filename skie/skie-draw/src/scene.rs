use crate::gpu::WHITE_TEX_ID;
use crate::paint::atlas::AtlasTextureInfoMap;
use crate::paint::DrawList;
use crate::paint::Mesh;
use crate::paint::PrimitiveKind;
use crate::paint::TextureId;
use crate::paint::Vertex;

#[derive(Debug, Clone)]
pub struct Primitive {
    pub kind: PrimitiveKind,
    pub texture: Option<TextureId>,
}

#[derive(Debug, Clone, Default)]
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

    pub fn add(&mut self, prim: impl Into<PrimitiveKind>, texture: Option<TextureId>) {
        self.items.push(Primitive {
            kind: prim.into(),
            texture,
        })
    }

    pub fn clear(&mut self) -> Vec<Primitive> {
        let old: Vec<Primitive> = std::mem::take(&mut self.items);
        old
    }

    pub fn get_required_textures(&self) -> impl Iterator<Item = TextureId> + '_ {
        self.items.iter().map(|f| f.texture.unwrap_or(WHITE_TEX_ID))
    }
    pub fn batches(&self, tex_info: AtlasTextureInfoMap) -> impl Iterator<Item = Mesh> + '_ {
        SceneBatchIterator::new(self, tex_info)
    }
}

// A simple batcher for now in future we will expand this.
struct SceneBatchIterator<'a> {
    scene: &'a Scene,
    groups: Vec<(Option<TextureId>, Vec<usize>)>,
    tex_info: AtlasTextureInfoMap,
    cur_group: usize,
}

impl<'a> SceneBatchIterator<'a> {
    pub fn new(scene: &'a Scene, tex_info: AtlasTextureInfoMap) -> Self {
        let mut tex_to_item_idx: ahash::AHashMap<Option<TextureId>, Vec<usize>> =
            Default::default();

        for (i, prim) in scene.items.iter().enumerate() {
            tex_to_item_idx.entry(prim.texture).or_default().push(i);
        }

        let mut groups: Vec<(Option<TextureId>, Vec<usize>)> =
            tex_to_item_idx.into_iter().collect();

        groups.sort_by_key(|(_, indices)| indices.first().copied().unwrap_or_default());

        log::trace!("Batches: {}", groups.len());
        Self {
            scene,
            tex_info,
            cur_group: 0,
            groups,
        }
    }
}

impl<'a> Iterator for SceneBatchIterator<'a> {
    type Item = Mesh;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_group >= self.groups.len() {
            return None;
        }

        let group = &self.groups[self.cur_group];
        let texture = group.0;

        let uv_middleware = |mut vertex: Vertex| {
            let texture = texture.unwrap_or(WHITE_TEX_ID);
            let info = self.tex_info.get(&texture);

            if let Some(info) = info {
                let [u, v] = vertex.uv;
                let (a_u, a_v) = info.uv_to_atlas_space(u, v);
                vertex.uv = [a_u, a_v];
            }

            vertex
        };

        let mut drawlist = DrawList::with_middleware(uv_middleware);

        for idx in &group.1 {
            let idx = *idx;
            let prim = &self.scene.items[idx];
            match &prim.kind {
                PrimitiveKind::Quad(quad) => drawlist.push_quad(quad),
            }
        }

        let mut mesh: Mesh = drawlist.into();
        if let Some(texture) = texture {
            mesh.texture = texture;
        }

        self.cur_group += 1;
        Some(mesh)
    }
}
