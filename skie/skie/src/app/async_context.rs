use std::rc::Weak;

use winit::window::WindowId;

use crate::{jobs::Jobs, window::Window};

use super::{AppContext, AppContextCell};
use anyhow::Result;

#[derive(Clone)]
pub struct AsyncAppContext {
    pub(crate) app: Weak<AppContextCell>,
    pub(crate) jobs: Jobs,
}

impl AsyncAppContext {
    pub fn update<R>(&self, cb: impl FnOnce(&mut AppContext) -> R) -> R {
        let cx = self.app.upgrade().expect("app released");
        let mut lock = cx.borrow_mut();
        lock.update(cb)
    }

    pub fn update_window<R, Update>(&self, id: &WindowId, update: Update) -> Result<R>
    where
        Update: FnOnce(&mut Window, &mut AppContext) -> R,
    {
        let cx = self.app.upgrade().expect("app released");
        let mut lock = cx.borrow_mut();
        lock.update_window(id, update)
    }
}
