use std::fmt::Debug;

use derive_more::Div;

use super::{Half, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Div)]
pub struct Size<T: std::fmt::Debug + Clone + Default> {
    pub width: T,
    pub height: T,
}

impl<T> Size<T>
where
    T: Clone + Default + Debug + Half,
{
    pub fn center(&self) -> Vec2<T> {
        Vec2 {
            x: self.width.half(),
            y: self.height.half(),
        }
    }
}

impl<T> Size<T>
where
    T: PartialOrd + Clone + Default + Debug,
{
    pub fn max(&self, other: &Self) -> Self {
        Size {
            width: if self.width >= other.width {
                self.width.clone()
            } else {
                other.width.clone()
            },
            height: if self.height >= other.height {
                self.height.clone()
            } else {
                other.height.clone()
            },
        }
    }

    pub fn min(&self, other: &Self) -> Self {
        Size {
            width: if self.width >= other.width {
                other.width.clone()
            } else {
                self.width.clone()
            },
            height: if self.height >= other.height {
                other.height.clone()
            } else {
                self.height.clone()
            },
        }
    }
}
