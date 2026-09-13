#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no GPU adapter: {0}")]
    NoAdapter(#[from] wgpu::RequestAdapterError),
    #[error("no GPU device: {0}")]
    NoDevice(#[from] wgpu::RequestDeviceError),
    #[error("surface: {0}")]
    Surface(#[from] wgpu::CreateSurfaceError),
    #[error("the surface offers no format")]
    SurfaceFormat,
    #[error("the glyph atlas is full")]
    AtlasFull,
    #[error("the frame changed under the text pass")]
    TextOutOfDate,
}

impl From<glyphon::PrepareError> for Error {
    fn from(_: glyphon::PrepareError) -> Self {
        Self::AtlasFull
    }
}

impl From<glyphon::RenderError> for Error {
    fn from(_: glyphon::RenderError) -> Self {
        Self::TextOutOfDate
    }
}

pub type Result<T> = std::result::Result<T, Error>;
