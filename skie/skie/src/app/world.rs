use std::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use slotmap::{SecondaryMap, SlotMap};

slotmap::new_key_type! {
    pub(crate) struct EntityId;
}

pub struct World {
    pub(crate) entities: SecondaryMap<EntityId, Box<dyn Any>>,
    // todo refcount
    pub(crate) ids: SlotMap<EntityId, ()>,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: SecondaryMap::new(),
            ids: Default::default(),
        }
    }

    pub fn insert<T>(&mut self, value: T) -> Entity<T>
    where
        T: 'static,
    {
        let id = self.ids.insert(());
        self.entities.insert(id, Box::new(value));
        Entity::new(id)
    }
}

pub struct Entity<T> {
    pub(crate) entity: AnyEntity,
    pub(crate) ty: PhantomData<T>,
}

impl<T: 'static> Entity<T> {
    fn new(id: EntityId) -> Self
    where
        T: 'static,
    {
        Self {
            entity: AnyEntity::new(id, TypeId::of::<T>()),
            ty: PhantomData,
        }
    }
}

pub struct AnyEntity {
    id: EntityId,
    ty: TypeId,
}

impl AnyEntity {
    fn new(id: EntityId, ty: TypeId) -> Self {
        Self { id, ty }
    }
}
