pub trait IsZero {
    fn is_zero(&self) -> bool;
}

impl IsZero for f64 {
    #[inline]
    fn is_zero(&self) -> bool {
        *self == 0.0
    }
}

impl IsZero for f32 {
    #[inline]
    fn is_zero(&self) -> bool {
        *self == 0.0
    }
}

impl IsZero for u32 {
    #[inline]
    fn is_zero(&self) -> bool {
        *self == 0
    }
}
