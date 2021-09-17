use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::num::NonZeroU32;
use std::path;
use std::path::Path;
use std::sync::Mutex;

use fast_image_resize as fr;
use image::{ColorType, GenericImageView};
use image::io::Reader as ImageReader;
use rayon::iter::*;
use structopt::StructOpt;
use tempfile::NamedTempFile;
use zip::{CompressionMethod, ZipWriter};
use zip::write::FileOptions;

use imagika::errors::ImageikaError;
use regex::Regex;
use lazy_static::lazy_static;

#[derive(Debug, StructOpt)]
#[structopt(name = "imagika", about = "A tool for compress pptx")]
struct Opts {
    #[structopt(short, long)]
    input: String,
    #[structopt(short, long)]
    output: String,
}

lazy_static! {
    static ref MEDIA_FILES: Regex = Regex::new(r"(?i)^ppt/media/.+").unwrap();
}


fn resize<P>(input: P, output: P) -> Result<(), ImageikaError> where P: AsRef<Path> {
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

    image::save_buffer_with_format(output,dst_image.get_buffer(), dst_width.get(), dst_height.get(), ColorType::Rgba8, format.unwrap())?;
    Ok(())
}

fn compress_pptx(input: String, output: String) -> std::result::Result<(), std::io::Error> {
    let input_path = path::Path::new(&input);
    let input_file = fs::File::open(input_path)?;
    let mut archive = zip::ZipArchive::new(input_file)?;
    let media: Vec<String> = archive.file_names()
        .filter(|file_name| MEDIA_FILES.is_match(file_name))
        .map(|s| String::from(s))
        .collect();
    let mut extracted: HashMap<String, NamedTempFile> = HashMap::new();
    let replacement: Mutex<HashMap<String, NamedTempFile>> = Mutex::new(HashMap::new());

    media.iter().for_each(|file_name| {
        if let Ok(mut file) = archive.by_name(file_name) {
            let mut tmp_file = tempfile::NamedTempFile::new().unwrap();
            let mut buf: Vec<u8> = Vec::new();
            file.read_to_end(&mut buf).unwrap();
            tmp_file.write_all(&buf).unwrap();
            tmp_file.flush().unwrap();
            extracted.insert(file_name.clone(), tmp_file);
        }
    });

    extracted.par_iter().for_each(|(file_name, input_temp_file)| {
        let output_temp_file = NamedTempFile::new().unwrap();
        let input_path_string = input_temp_file.path().to_string_lossy().to_string();
        let output_path_string = output_temp_file.path().to_string_lossy().to_string();
        match resize(input_path_string, output_path_string) {
            Ok(_) => {
                replacement.lock().unwrap().insert(file_name.clone(), output_temp_file);
                println!("Compress {} success", file_name);
            }
            Err(e) => {
                println!("Compress {} failed: {:?}", file_name, e);
            }
        }
    });

    let output_file = std::fs::File::create(output)?;
    let mut writer = ZipWriter::new(output_file);
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let mut replacement = replacement.lock().unwrap();
        let options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .last_modified_time(file.last_modified())
            .unix_permissions(file.unix_mode().unwrap_or_default());
        if replacement.contains_key(file.name()) {
            writer.start_file(file.name(), options).unwrap();
            let image = replacement.get_mut(file.name()).unwrap();
            let mut buf: Vec<u8> = Vec::new();
            image.read_to_end(&mut buf).unwrap();
            writer.write_all(&buf).unwrap();
        } else {
            writer.start_file(file.name(), options).unwrap();
            let mut buf: Vec<u8> = Vec::new();
            file.read_to_end(&mut buf).unwrap();
            writer.write_all(&buf).unwrap();
        }
    }
    writer.finish()?;
    Ok(())
}

fn main() {
    let opts: Opts = Opts::from_args();

    println!("Thread pool size: {}", rayon::current_num_threads());

    match compress_pptx(opts.input, opts.output) {
        Ok(_) => {
            println!("Finished");
        }
        Err(e) => {
            println!("Failed:\n{}", e);
        }
    }
}
