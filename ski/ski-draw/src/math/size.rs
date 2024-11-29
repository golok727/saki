#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size<T: std::fmt::Debug + Clone + Default> {
    width: T,
    height: T,
}
