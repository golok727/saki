#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rect<T: std::fmt::Debug + Clone + Default> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}
