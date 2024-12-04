#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct CreateWindowError(#[from] pub winit::error::OsError);
