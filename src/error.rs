use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptionError {
    #[error("image decode error: {0}")]
    ImageDecode(#[from] image::ImageError),

    #[error("gif encode error: {0}")]
    GifEncode(#[from] gif::EncodingError),

    #[error("gif decode error: {0}")]
    GifDecode(#[from] gif::DecodingError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("empty image")]
    EmptyImage,
}
