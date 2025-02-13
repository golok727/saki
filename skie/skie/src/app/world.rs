use std::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use derive_more::derive::{Deref, DerefMut};
use slotmap::{SecondaryMap, SlotMap};

use super::AppContext;

slotmap::new_key_type! {
    pub struct EntityId;
}

pub struct World {
    pub(crate) entities: SecondaryMap<EntityId, Box<dyn Any>>,
    // todo refcount
    pub(crate) ids: SlotMap<EntityId, ()>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn read<T: 'static>(&self, entity: &Entity<T>) -> &T {
        self.entities
            .get(entity.id)
            .and_then(|ent| ent.downcast_ref())
            .expect("Error reading entity")
    }

    pub fn detach<'a, T>(&mut self, handle: &'a Entity<T>) -> DetachedEntity<'a, T> {
        let value = Some(
            self.entities
                .remove(handle.id)
                .expect("entity already detached"),
        );
        DetachedEntity {
            value,
            handle,
            ty: PhantomData,
        }
    }
}

#[derive(Clone, Deref, DerefMut)]
pub struct Entity<T> {
    #[deref]
    #[deref_mut]
    pub(crate) any_entity: AnyEntity,
    pub(crate) ty: PhantomData<T>,
}

impl<T: 'static> Entity<T> {
    fn new(id: EntityId) -> Self
    where
        T: 'static,
    {
        Self {
            any_entity: AnyEntity::new(id, TypeId::of::<T>()),
            ty: PhantomData,
        }
    }

    pub fn id(&self) -> EntityId {
        self.any_entity.id
    }

    pub fn into_any(self) -> AnyEntity {
        self.any_entity
    }

    pub fn read<'a>(&self, cx: &'a AppContext) -> &'a T {
        cx.world.read(self)
    }

    pub fn update<R>(
        &self,
        cx: &mut AppContext,
        update: impl FnOnce(&mut T, &mut AppContext) -> R,
    ) -> R {
        cx.update_entity(self, update)
    }
}

#[derive(Clone, Debug)]
pub struct AnyEntity {
    id: EntityId,
    ty: TypeId,
}

impl AnyEntity {
    fn new(id: EntityId, ty: TypeId) -> Self {
        Self { id, ty }
    }

    pub fn downcast<T: 'static>(self) -> Result<Entity<T>, AnyEntity> {
        if TypeId::of::<T>() == self.ty {
            Ok(Entity {
                any_entity: self,
                ty: PhantomData,
            })
        } else {
            Err(self)
        }
    }
}

pub(crate) struct DetachedEntity<'a, T> {
    value: Option<Box<dyn Any>>,
    pub handle: &'a Entity<T>,
    ty: PhantomData<T>,
}

impl<'a, T: 'static> DetachedEntity<'a, T> {
    pub fn attach(mut self, world: &mut World) {
        world
            .entities
            .insert(self.handle.id, self.value.take().unwrap());
    }
}

impl<'a, T: 'static> core::ops::Deref for DetachedEntity<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.as_ref().unwrap().downcast_ref().unwrap()
    }
}

impl<'a, T: 'static> core::ops::DerefMut for DetachedEntity<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value.as_mut().unwrap().downcast_mut().unwrap()
    }
}

impl<'a, T> Drop for DetachedEntity<'a, T> {
    fn drop(&mut self) {
        if self.value.is_some() && !std::thread::panicking() {
            panic!("Attach back the entity")
        }
    }
}
