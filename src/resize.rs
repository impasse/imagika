use std::num::NonZeroU32;
use std::path::Path;

use fast_image_resize as fr;
use image::{ColorType, GenericImageView};
use image::io::Reader as ImageReader;

use crate::errors::ImageikaError;

pub fn resize<P>(input: P, output: P) -> Result<(), ImageikaError> where P: AsRef<Path> {
    let reader = ImageReader::open(input)?
        .with_guessed_format()?;
    let format = reader.format();
    let img = reader.decode()?;
    let src_width = NonZeroU32::new(img.width()).unwrap();
    let src_height = NonZeroU32::new(img.height()).unwrap();
    let mut src_image = fr::ImageData::from_vec_u8(
        src_width,
        src_height,
        img.to_rgba8().into_raw(),
        fr::PixelType::U8x4,
    )?;
    let alpha_mul_div: fr::MulDiv = Default::default();

    // Multiple RGB channels of source image by alpha channel
    alpha_mul_div.multiply_alpha_inplace(&mut src_image.dst_view())?;

    let ratio = img.width() as f32 / img.height() as f32;
    let base = 1000 as f32;

    // Create wrapper that own data of destination image
    let dst_width = NonZeroU32::new(base as u32).unwrap();
    let dst_height = NonZeroU32::new((base / ratio) as u32).unwrap();
    let mut dst_image = fr::ImageData::new(dst_width, dst_height, src_image.pixel_type());

    // Get mutable view of destination image data
    let mut dst_view = dst_image.dst_view();

    // Create Resizer instance and resize source image
    // into buffer of destination image
    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3));
    resizer.resize(&src_image.src_view(), &mut dst_view);

    // Divide RGB channels of destination image by alpha
    alpha_mul_div.divide_alpha_inplace(&mut dst_view)?;

    image::save_buffer_with_format(output, dst_image.get_buffer(), dst_width.get(), dst_height.get(), ColorType::Rgba8, format.unwrap())?;
    Ok(())
}