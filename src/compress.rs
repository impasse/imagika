use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;

use lazy_static::lazy_static;
use rayon::iter::*;
use regex::Regex;
use tempfile::NamedTempFile;
use zip::{CompressionMethod, ZipWriter};
use zip::write::FileOptions;

use crate::resize;

lazy_static! {
    static ref MEDIA_FILES: Regex = Regex::new(r"(?i)^ppt/media/.+").unwrap();
}

pub fn compress_pptx(input: String, output: String) -> std::result::Result<(), std::io::Error> {
    let input_path = Path::new(&input);
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