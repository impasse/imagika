use fast_image_resize::{ImageBufferError, MulDivImageError};
use image::ImageError;

#[derive(Debug)]
pub enum ImageikaError {
    IoError(std::io::Error),
    ImageError(ImageError),
    ImageBufferError(ImageBufferError),
    MulDivImageError(MulDivImageError),
}

impl From<std::io::Error> for ImageikaError {
    fn from(e: std::io::Error) -> Self {
        ImageikaError::IoError(e)
    }
}

impl From<ImageError> for ImageikaError {
    fn from(e: ImageError) -> Self {
        ImageikaError::ImageError(e)
    }
}

impl From<ImageBufferError> for ImageikaError {
    fn from(e: ImageBufferError) -> Self {
        ImageikaError::ImageBufferError(e)
    }
}

impl From<MulDivImageError> for ImageikaError {
    fn from(e: MulDivImageError) -> Self {
        ImageikaError::MulDivImageError(e)
    }
}