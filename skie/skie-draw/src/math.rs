pub mod geometry;
pub mod rect;
pub mod size;
pub mod unit;

pub use geometry::*;
pub use rect::*;
pub use size::*;
pub use unit::*;

pub use unit::{DevicePixels, ScaledPixels};

use crate::traits::{IsZero, One, Zero};

pub trait Half {
    fn half(&self) -> Self;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Mat3 {
    data: [f32; 9],
}

impl Mat3 {
    /**
    constructs a identity matrix
    */
    #[inline]
    pub fn new() -> Self {
        Self {
            #[rustfmt::skip]
            data: [
                1.0, 0.0, 0.0,
                0.0, 1.0, 0.0,
                0.0, 0.0, 1.0
            ],
        }
    }

    #[inline]
    pub fn identity() -> Self {
        Self::new()
    }

    #[inline]
    pub fn from_scale(sx: f32, sy: f32) -> Self {
        let mut mat = Self::new();
        mat.scale(sx, sy);
        mat
    }

    #[inline]
    pub fn from_translation(dx: f32, dy: f32) -> Self {
        let mut mat = Self::new();
        mat.translate(dx, dy);
        mat
    }

    #[inline]
    pub fn translate(&mut self, dx: f32, dy: f32) -> &mut Self {
        self.data[2] += dx;
        self.data[5] += dy;
        self
    }

    #[inline]
    pub fn translate_x(&mut self, dx: f32) -> &mut Self {
        self.translate(dx, 0.)
    }

    #[inline]
    pub fn translate_y(&mut self, dy: f32) -> &mut Self {
        self.translate(0., dy)
    }

    #[inline]
    pub fn scale(&mut self, sx: f32, sy: f32) -> &mut Self {
        self.data[0] *= sx;
        self.data[4] *= sy;

        self.data[1] *= sx;
        self.data[3] *= sy;

        self.data[2] *= sx;
        self.data[5] *= sy;

        self
    }

    #[inline]
    pub fn scale_x(&mut self, sx: f32) -> &mut Self {
        self.scale(sx, 1.)
    }

    #[inline]
    pub fn scale_y(&mut self, sy: f32) -> &mut Self {
        self.scale(1., sy)
    }

    pub fn transpose(&mut self) -> &mut Self {
        self.data.swap(1, 3);
        self.data.swap(2, 6);
        self.data.swap(5, 7);
        self
    }

    pub fn inverse(&self) -> Self {
        let m = &self.data;

        let det = self.det();

        if det == 0.0 {
            return *self;
        }

        let inv_det = 1.0 / det;

        Self {
            data: [
                (m[4] * m[8] - m[5] * m[7]) * inv_det,
                (m[2] * m[7] - m[1] * m[8]) * inv_det,
                (m[1] * m[5] - m[2] * m[4]) * inv_det,
                (m[5] * m[6] - m[3] * m[8]) * inv_det,
                (m[0] * m[8] - m[2] * m[6]) * inv_det,
                (m[2] * m[3] - m[0] * m[5]) * inv_det,
                (m[3] * m[7] - m[4] * m[6]) * inv_det,
                (m[1] * m[6] - m[0] * m[7]) * inv_det,
                (m[0] * m[4] - m[1] * m[3]) * inv_det,
            ],
        }
    }

    pub fn det(&self) -> f32 {
        let m = &self.data;

        // Determinant formula for a 3x3 matrix:
        // | a b c |
        // | d e f |
        // | g h i |
        // det = a(ei - fh) - b(di - fg) + c(dh - eg)

        m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
            + m[2] * (m[3] * m[7] - m[4] * m[6])
    }

    pub fn is_identity(&self) -> bool {
        let m = &self.data;
        m[0] == 1.
            && m[4] == 1.
            && m[8] == 1.
            && m[1] == 0.
            && m[2] == 0.
            && m[3] == 0.
            && m[5] == 0.
            && m[6] == 0.
            && m[7] == 0.
    }

    /// Constructs an orthographic projection matrix
    pub fn ortho(top: f32, left: f32, bottom: f32, right: f32) -> Self {
        let scale_x = 2.0 / (right - left);
        let scale_y = 2.0 / (top - bottom);
        let translate_x = -(right + left) / (right - left);
        let translate_y = -(top + bottom) / (top - bottom);

        Self {
            #[rustfmt::skip]
            data: [
                scale_x, 0.0, translate_x,
                0.0, scale_y, translate_y,
                0.0, 0.0, 1.0,
            ],
        }
    }
}

impl From<Mat3> for [[f32; 4]; 4] {
    #[rustfmt::skip]
    fn from(mat: Mat3) -> Self {
        let m = mat.data;
        [
            [m[0], m[1], m[2], 0.0], // Row 0
            [m[3], m[4], m[5], 0.0], // Row 1
            [m[6], m[7], m[8], 0.0], // Row 2
            [0.0, 0.0, 0.0, 1.0],    // Row 3
        ]
    }
}

#[inline]
pub fn mat3() -> Mat3 {
    Mat3::new()
}

impl Default for Mat3 {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Mul for Mat3 {
    type Output = Self;

    /*
    a b tx   m11 m12 m13
    c d ty   m21 m22 m23
    0 0 1    m31 m32 m33
    */
    fn mul(self, rhs: Self) -> Self::Output {
        let a = &rhs.data;
        let b = &self.data;

        let m11 = a[0] * b[0] + a[1] * b[3] + a[2] * b[6];
        let m12 = a[0] * b[1] + a[1] * b[4] + a[2] * b[7];
        let m13 = a[0] * b[2] + a[1] * b[5] + a[2] * b[8];

        let m21 = a[3] * b[0] + a[4] * b[3] + a[5] * b[6];
        let m22 = a[3] * b[1] + a[4] * b[4] + a[5] * b[7];
        let m23 = a[3] * b[2] + a[4] * b[5] + a[5] * b[8];

        let m31 = a[6] * b[0] + a[7] * b[3] + a[8] * b[6];
        let m32 = a[6] * b[1] + a[7] * b[4] + a[8] * b[7];
        let m33 = a[6] * b[2] + a[7] * b[5] + a[8] * b[8];

        Self {
            data: [m11, m12, m13, m21, m22, m23, m31, m32, m33],
        }
    }
}

// :) we just need a vec2 for or needs!
impl std::ops::Mul<Vec2<f32>> for Mat3 {
    type Output = Vec2<f32>;

    fn mul(self, v: Vec2<f32>) -> Self::Output {
        let m = &self.data;
        let x = m[0] * v.x + m[1] * v.y + m[2] * 1.0;
        let y = m[3] * v.x + m[4] * v.y + m[5] * 1.0;

        Self::Output { x, y }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

impl<T> Vec2<T> {
    #[inline]
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T> Vec2<T>
where
    T: Clone + std::ops::Add<T, Output = T> + std::ops::Mul<T, Output = T>,
{
    pub fn dot(&self, other: &Self) -> T {
        self.x.clone() * other.x.clone() + self.y.clone() * other.y.clone()
    }
}

impl<T> Vec2<T>
where
    T: Clone
        + std::ops::Add<T, Output = T>
        + std::ops::Mul<T, Output = T>
        + std::ops::Sub<T, Output = T>,
{
    pub fn cross(&self, other: &Self) -> T {
        self.x.clone() * other.y.clone() - self.y.clone() * other.x.clone()
    }
}

impl From<Vec2<f32>> for [f32; 4] {
    fn from(Vec2 { x, y }: Vec2<f32>) -> Self {
        [x, y, 1.0, 1.0]
    }
}

impl<T> From<Vec2<T>> for (T, T) {
    fn from(Vec2 { x, y }: Vec2<T>) -> Self {
        (x, y)
    }
}

impl<T> From<(T, T)> for Vec2<T> {
    fn from((x, y): (T, T)) -> Self {
        Self { x, y }
    }
}

impl From<Vec2<f32>> for [f32; 2] {
    fn from(Vec2 { x, y }: Vec2<f32>) -> Self {
        [x, y]
    }
}

impl<T> Half for Vec2<T>
where
    T: Half,
{
    fn half(&self) -> Self {
        Self {
            x: self.x.half(),
            y: self.y.half(),
        }
    }
}

impl<T> IsZero for Vec2<T>
where
    T: IsZero,
{
    fn is_zero(&self) -> bool {
        self.x.is_zero() && self.y.is_zero()
    }
}

impl<T> Zero for Vec2<T>
where
    T: Zero,
{
    fn zero() -> Self {
        Self {
            x: T::zero(),
            y: T::zero(),
        }
    }
}

impl<T> One for Vec2<T>
where
    T: One,
{
    fn one() -> Self {
        Self {
            x: T::one(),
            y: T::one(),
        }
    }
}

macro_rules! impl_vec2_common {
    ($t:ty) => {
        impl Vec2<$t> {}
    };
}

impl_vec2_common!(u8);
impl_vec2_common!(u16);
impl_vec2_common!(u32);
impl_vec2_common!(u64);

impl_vec2_common!(i8);
impl_vec2_common!(i16);
impl_vec2_common!(i32);
impl_vec2_common!(i64);

impl_vec2_common!(f64);
impl_vec2_common!(f32);

pub trait OneVec<T> {
    fn one() -> Vec2<T>;
}

macro_rules! impl_vec2_float {
    ($float:ty) => {
        impl Vec2<$float> {
            pub fn magnitude_sq(&self) -> $float {
                self.x * self.x + self.y * self.y
            }

            pub fn magnitude(&self) -> $float {
                (self.x * self.x + self.y * self.y).sqrt()
            }

            pub fn normalize(&self) -> Self {
                let len = (self.x * self.x + self.y * self.y).sqrt();
                if len == 0.0 {
                    Self::zero()
                } else {
                    Self {
                        x: self.x / len,
                        y: self.y / len,
                    }
                }
            }

            pub fn direction(&self, other: Self) -> Self {
                (*self - other).normalize()
            }

            pub fn angle(v1: Self, v2: Self) -> $float {
                let dot = v1.x * v2.x + v1.y * v2.y;
                let det = v1.x * v2.y - v1.y * v2.x;
                det.atan2(dot).abs()
            }
        }
    };
}

impl_vec2_float!(f32);
impl_vec2_float!(f64);

impl<T> std::cmp::PartialEq for Vec2<T>
where
    T: std::cmp::PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

// Vector
impl<T> std::ops::Add for Vec2<T>
where
    T: std::ops::Add<T, Output = T>,
{
    type Output = Vec2<T>;

    fn add(self, rhs: Vec2<T>) -> Self::Output {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

// Scalar
impl<T> std::ops::Add<T> for Vec2<T>
where
    T: Clone + std::ops::Add<T, Output = T>,
{
    type Output = Vec2<T>;

    fn add(self, scalar: T) -> Self::Output {
        Vec2 {
            x: self.x + scalar.clone(),
            y: self.y + scalar.clone(),
        }
    }
}

// Begin Vector assign
impl<T> std::ops::AddAssign for Vec2<T>
where
    T: std::ops::Add<T, Output = T> + Clone,
{
    fn add_assign(&mut self, rhs: Self) {
        *self = self.clone() + rhs
    }
}

impl<T> std::ops::SubAssign for Vec2<T>
where
    T: std::ops::Sub<T, Output = T> + Clone,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.clone() - rhs
    }
}

impl<T> std::ops::MulAssign for Vec2<T>
where
    T: std::ops::Mul<T, Output = T> + Clone,
{
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.clone() * rhs
    }
}

impl<T> std::ops::DivAssign for Vec2<T>
where
    T: std::ops::Div<T, Output = T> + Clone,
{
    fn div_assign(&mut self, rhs: Self) {
        *self = self.clone() / rhs
    }
}
// END Vector assign

// Scalar
impl<T> std::ops::Sub<T> for Vec2<T>
where
    T: Clone + std::ops::Sub<T, Output = T>,
{
    type Output = Vec2<T>;

    fn sub(self, scalar: T) -> Self::Output {
        Vec2 {
            x: self.x - scalar.clone(),
            y: self.y - scalar.clone(),
        }
    }
}

// Vector
impl<T> std::ops::Sub for Vec2<T>
where
    T: std::ops::Sub<T, Output = T>,
{
    type Output = Vec2<T>;

    fn sub(self, rhs: Vec2<T>) -> Self::Output {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

// Vector
impl<T> std::ops::Mul for Vec2<T>
where
    T: std::ops::Mul<T, Output = T>,
{
    type Output = Vec2<T>;

    fn mul(self, rhs: Vec2<T>) -> Self::Output {
        Vec2 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

// Scalar
impl<T> std::ops::Mul<T> for Vec2<T>
where
    T: Clone + std::ops::Mul<T, Output = T>,
{
    type Output = Vec2<T>;

    fn mul(self, scalar: T) -> Self::Output {
        Vec2 {
            x: self.x * scalar.clone(),
            y: self.y * scalar.clone(),
        }
    }
}

// Vector
impl<T> std::ops::Div for Vec2<T>
where
    T: std::ops::Div<T, Output = T>,
{
    type Output = Vec2<T>;

    fn div(self, rhs: Vec2<T>) -> Self::Output {
        Vec2 {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}

// Scalar
impl<T> std::ops::Div<T> for Vec2<T>
where
    T: Clone + std::ops::Div<T, Output = T>,
{
    type Output = Vec2<T>;

    fn div(self, scalar: T) -> Self::Output {
        Vec2 {
            x: self.x / scalar.clone(),
            y: self.y / scalar.clone(),
        }
    }
}

#[inline]
pub fn vec2<T>(x: T, y: T) -> Vec2<T> {
    Vec2 { x, y }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod mat3 {
        use super::*;

        #[test]
        fn should_multiply() {
            let m1 = mat3();
            let m2 = mat3();

            let c = m1 * m2;
            assert_eq!(c, mat3());
        }

        #[test]
        fn compose_matrices() {
            let scale = Mat3::from_scale(10.0, 10.0);
            let translate = Mat3::from_translation(100.0, 100.0);

            let res = scale * translate * vec2(10.0, 10.0);

            assert_eq!(res, vec2(200.0, 200.0));
        }

        #[test]
        fn matrix_transform() {
            let mut transform = mat3();
            transform
                .scale(10.0, 10.0)
                .translate(100.0, 100.0)
                .scale(10.0, 10.0);

            let res = transform * vec2(10.0, 10.0);

            assert_eq!(res, vec2(2000.0, 2000.0));
        }

        #[test]
        fn orthographic_projection() {
            let m = Mat3::ortho(0.0, 0.0, 100.0, 100.0);

            assert_eq!(m * vec2(50.0, 50.0), vec2(0.0, 0.0)); // center
            assert_eq!(m * vec2(0.0, 0.0), vec2(-1.0, 1.0)); // top left
            assert_eq!(m * vec2(100.0, 0.0), vec2(1.0, 1.0)); // top right
            assert_eq!(m * vec2(0.0, 100.0), vec2(-1.0, -1.0)); // bottom left
            assert_eq!(m * vec2(100.0, 100.0), vec2(1.0, -1.0)); // bottom right
        }

        #[test]
        fn triangle_proj_test() {
            let width: u32 = 1875;
            let height: u32 = 1023;

            let aspect: f32 = width as f32 / height as f32;
            let proj = Mat3::ortho(1.0, aspect, -1.0, -aspect);

            let positions = [vec2(-0.5, -0.5), vec2(0.0, 0.5), vec2(0.5, -0.5)];
            let transformed = positions.map(|v| proj * v);

            assert_eq!(
                [vec2(0.2728, -0.5), vec2(0.0, 0.5), vec2(-0.2728, -0.5)],
                transformed
            );
        }

        #[test]
        fn is_identity() {
            assert!(mat3().is_identity())
        }
    }
    mod vec2 {
        use super::*;

        #[test]
        fn zero_and_one() {
            assert_eq!(Vec2::<f64>::zero(), vec2(0.0, 0.0));
            assert_eq!(Vec2::<f64>::one(), vec2(1.0, 1.0));
        }
        #[test]
        fn vec_add() {
            let a = vec2(10.0, 10.0);
            let b = vec2(10.0, 10.0);

            assert_eq!(a + b, vec2(20.0, 20.0));
        }

        #[test]
        fn vec_add_assign() {
            let mut a = vec2(10.0, 10.0);
            a += vec2(10.0, 10.0);

            assert_eq!(a, vec2(20.0, 20.0));
        }

        #[test]
        fn vec_sub() {
            let a = vec2(10.0, 10.0);
            let b = vec2(10.0, 10.0);

            assert_eq!(a - b, vec2(0.0, 0.0));
        }

        #[test]
        fn vec_sub_assign() {
            let mut a = vec2(10.0, 10.0);
            a -= vec2(10.0, 10.0);

            assert_eq!(a, vec2(0.0, 0.0));
        }

        #[test]
        fn vec_mul() {
            let a = vec2(10.0, 10.0);
            let b = vec2(10.0, 10.0);

            assert_eq!(a * b, vec2(100.0, 100.0));
        }

        #[test]
        fn vec_mul_assign() {
            let mut a = vec2(10.0, 10.0);
            a *= vec2(10.0, 10.0);

            assert_eq!(a, vec2(100.0, 100.0));
        }

        #[test]
        fn vec_div() {
            let a = vec2(10.0, 10.0);
            let b = vec2(10.0, 10.0);

            assert_eq!(a / b, vec2(1.0, 1.0));
        }

        #[test]
        fn vec_div_assign() {
            let mut a = vec2(10.0, 10.0);
            a /= vec2(10.0, 10.0);

            assert_eq!(a, vec2(1.0, 1.0));
        }

        #[test]
        fn should_transform_with_matrix() {
            let mut m = mat3();
            m.translate(10.0, 100.0);
            m.translate(20.0, 100.0);

            let a = vec2(10.0, 0.0);
            assert_eq!(m * a, vec2(40.0, 200.0));
        }

        #[test]
        fn should_scale_with_matrix() {
            let mut m = mat3();
            m.scale(2.0, 2.0).scale(2.0, 2.0);

            let a = vec2(10.0, 50.0);

            assert_eq!(m * a, vec2(40.0, 200.0));
        }
    }
}
