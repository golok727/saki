use derive_more::{Add, AddAssign, Display, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    Display,
    PartialEq,
    PartialOrd,
    Add,
    AddAssign,
    Mul,
    MulAssign,
    Sub,
    SubAssign,
    Div,
    DivAssign,
    Neg,
)]
#[repr(transparent)]
#[display("{_0}px")]

pub struct Pixels(pub(crate) f32);

impl Pixels {
    pub fn floor(&self) -> Self {
        Self(self.0.floor())
    }

    pub fn round(&self) -> Self {
        Self(self.0.round())
    }

    pub fn ceil(&self) -> Self {
        Self(self.0.ceil())
    }

    pub fn scale(&self, scale_factor: f32) -> ScaledPixels {
        ScaledPixels(self.0 * scale_factor)
    }

    pub fn pow(&self, exponent: f32) -> Self {
        Self(self.0.powf(exponent))
    }

    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    pub fn sign(&self) -> f32 {
        self.0.signum()
    }

    pub fn to_f64(self) -> f64 {
        self.0 as f64
    }
}

#[inline]
pub fn px(val: impl Into<Pixels>) -> Pixels {
    val.into()
}

macro_rules! impl_from_as {
    ($f:ty, $recv:ty, $as:ty) => {
        impl From<$f> for $recv {
            fn from(value: $f) -> Self {
                Self(value as $as)
            }
        }
    };
}

impl_from_as!(u8, Pixels, f32);
impl_from_as!(u16, Pixels, f32);
impl_from_as!(u32, Pixels, f32);
impl_from_as!(u64, Pixels, f32);

impl_from_as!(i8, Pixels, f32);
impl_from_as!(i16, Pixels, f32);
impl_from_as!(i32, Pixels, f32);
impl_from_as!(i64, Pixels, f32);

impl_from_as!(f32, Pixels, f32);
impl_from_as!(f64, Pixels, f32);

/// ScreenPixels: Pixels that are tied to the screen resolution (e.g., after scaling)
#[derive(
    Debug,
    Default,
    Display,
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    Add,
    AddAssign,
    Mul,
    MulAssign,
    Sub,
    SubAssign,
    Div,
    DivAssign,
    Neg,
)]
#[repr(transparent)]
#[display("{_0}spx")]
pub struct ScaledPixels(pub(crate) f32);

impl_from_as!(u8, ScaledPixels, f32);
impl_from_as!(u16, ScaledPixels, f32);
impl_from_as!(u32, ScaledPixels, f32);
impl_from_as!(u64, ScaledPixels, f32);

impl_from_as!(i8, ScaledPixels, f32);
impl_from_as!(i16, ScaledPixels, f32);
impl_from_as!(i32, ScaledPixels, f32);
impl_from_as!(i64, ScaledPixels, f32);

impl_from_as!(f32, ScaledPixels, f32);
impl_from_as!(f64, ScaledPixels, f32);

/// DevicePixels: Pixels in device-specific resolution
#[derive(
    Debug,
    Default,
    Display,
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    Add,
    AddAssign,
    Mul,
    MulAssign,
    Sub,
    SubAssign,
    Div,
    DivAssign,
    Neg,
)]
#[repr(transparent)]
#[display("{_0}dpx")]
pub struct DevicePixels(pub(crate) i32);

impl_from_as!(u8, DevicePixels, i32);
impl_from_as!(u16, DevicePixels, i32);
impl_from_as!(u32, DevicePixels, i32);
impl_from_as!(u64, DevicePixels, i32);

impl_from_as!(i8, DevicePixels, i32);
impl_from_as!(i16, DevicePixels, i32);
impl_from_as!(i32, DevicePixels, i32);
impl_from_as!(i64, DevicePixels, i32);

impl_from_as!(f32, DevicePixels, i32);
impl_from_as!(f64, DevicePixels, i32);

impl DevicePixels {
    /// Converts DevicePixels to ScreenPixels based on a scale factor (e.g., from device to screen)
    pub fn scale(self, scale_factor: f32) -> ScaledPixels {
        let scaled_value = self.0 as f32 * scale_factor;
        ScaledPixels(scaled_value)
    }

    /// Converts ScreenPixels to DevicePixels based on a scale factor (e.g., from screen back to device)
    pub fn from_scaled(self, scale_factor: f32) -> DevicePixels {
        let original_value = (self.0 as f32 / scale_factor) as i32;
        DevicePixels(original_value)
    }
}

#[inline]
pub fn device_px(val: impl Into<DevicePixels>) -> DevicePixels {
    val.into()
}
