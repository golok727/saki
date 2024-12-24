use std::fmt::Debug;

use crate::traits::{IsZero, Zero};

use super::{Half, Size, Vec2};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rect<T: Debug + Clone + Default> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl<T: Debug + Clone + Default> From<(T, T, T, T)> for Rect<T> {
    fn from(v: (T, T, T, T)) -> Self {
        Self {
            x: v.0,
            y: v.1,
            width: v.2,
            height: v.3,
        }
    }
}
impl<T: Debug + Clone + Default> Rect<T> {
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

impl<T> Zero for Rect<T>
where
    T: Zero + Debug + Clone + Default,
{
    fn zero() -> Self {
        Self {
            x: T::zero(),
            y: T::zero(),
            width: T::zero(),
            height: T::zero(),
        }
    }

    fn to_zero(&mut self) {
        self.width = T::zero();
        self.height = T::zero();
    }
}

impl<T> Half for Rect<T>
where
    T: Half + Debug + Clone + Default,
{
    fn half(&self) -> Self {
        Self {
            x: self.x.clone(),
            y: self.y.clone(),
            width: self.width.half(),
            height: self.height.half(),
        }
    }
}

impl<T> Rect<T>
where
    T: IsZero + Debug + Clone + Default,
{
    /// returns if the width and height are zero
    pub fn empty(&self) -> bool {
        self.width.is_zero() && self.height.is_zero()
    }
}

impl<T> Rect<T>
where
    T: Debug + Copy + Default + PartialOrd,
{
    pub fn union(&mut self, other: &Self) {
        self.x = if other.x < self.x { other.x } else { self.x };
        self.y = if other.y < self.y { other.y } else { self.y };
        self.width = if other.width > self.width {
            other.width
        } else {
            self.width
        };
        self.height = if other.height > self.height {
            other.height
        } else {
            self.height
        };
    }
}

impl<T> Rect<T>
where
    T: Debug
        + Clone
        + Default
        + std::cmp::PartialOrd<T>
        + std::ops::Add<T, Output = T>
        + std::ops::Sub<T, Output = T>,
{
    pub fn include_point(&mut self, p: &Vec2<T>) {
        let x = self.x.clone();
        let y = self.y.clone();
        let width = self.width.clone();
        let height = self.height.clone();

        if p.x < x {
            self.x = p.x.clone();
        }

        if p.x > x.clone() + width.clone() {
            self.width = p.x.clone() - x.clone();
        }

        if p.y < y {
            self.y = p.y.clone();
        }

        if p.y > y.clone() + height.clone() {
            self.height = p.y.clone() - y.clone();
        }
    }
}

impl<T> Rect<T>
where
    T: Debug + Clone + Default + std::ops::Add<T, Output = T>,
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
