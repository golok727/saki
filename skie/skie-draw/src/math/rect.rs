use super::{Size, Vec2};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rect<T: std::fmt::Debug + Clone + Default> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl<T: std::fmt::Debug + Clone + Default> Rect<T> {
    pub fn position(&self) -> Vec2<T> {
        Vec2 {
            x: self.x.clone(),
            y: self.y.clone(),
        }
    }

    pub fn size(&self) -> Size<T> {
        Size {
            width: self.width.clone(),
            height: self.height.clone(),
        }
    }
}

impl<T> Rect<T>
where
    T: std::fmt::Debug + Clone + Default + std::ops::Add<T, Output = T>,
{
    pub fn min(&self) -> Vec2<T> {
        Vec2 {
            x: self.x.clone(),
            y: self.y.clone(),
        }
    }

    pub fn max(&self) -> Vec2<T> {
        Vec2 {
            x: self.x.clone(),
            y: self.y.clone(),
        }
    }

    pub fn top_left(&self) -> Vec2<T> {
        Vec2 {
            x: self.x.clone(),
            y: self.y.clone(),
        }
    }

    pub fn top_right(&self) -> Vec2<T> {
        Vec2 {
            x: self.x.clone() + self.width.clone(),
            y: self.y.clone(),
        }
    }

    pub fn bottom_left(&self) -> Vec2<T> {
        Vec2 {
            x: self.x.clone(),
            y: self.y.clone() + self.height.clone(),
        }
    }

    pub fn bottom_right(&self) -> Vec2<T> {
        Vec2 {
            x: self.x.clone() + self.width.clone(),
            y: self.y.clone() + self.height.clone(),
        }
    }
}
