use std::marker::PhantomData;

use derive_more::derive::{Deref, DerefMut};

use super::AppContext;

#[derive(Deref, DerefMut)]
pub struct Context<'a, T> {
    #[deref]
    #[deref_mut]
    app: &'a mut AppContext,
    ty: PhantomData<T>,
}

impl<'a, T: 'static> Context<'a, T> {}
