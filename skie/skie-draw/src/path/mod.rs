mod p2d;
pub use p2d::*;

#[derive(Debug, Clone, Copy, Hash, PartialEq)]
enum Verb {
    QuadTo,
    LineTo,
    CubeTo,
    Close,
}
