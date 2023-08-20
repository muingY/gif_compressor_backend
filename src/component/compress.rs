use std::fs::File;
use std::path::Path;
use gif::{Encoder, Frame, Repeat};
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, ImageDecoder};
use image::imageops::FilterType;
use crate::component::CompressErrType::{CompressFail, FileSystemFail};

#[derive(Debug)]
pub enum CompressErrType {
    CompressFail = 0,
    FileSystemFail = 1,
}

pub async fn compress(
    gif_path: String,
    result_save_dir: String,
    result_filename_suffix: String,
    // TODO: Add compress option struct later...
) -> Result<String, CompressErrType> {
    let file_in: File;
    match File::open(gif_path.clone()) {
        Ok(result) => {
            file_in = result;
        }
        Err(_) => {
            return Err(FileSystemFail);
        }
    }
    let decoder: GifDecoder<File>;
    match GifDecoder::new(file_in) {
        Ok(result) => {
            decoder = result;
        }
        Err(_) => {
            return Err(CompressFail);
        }
    }
    let (width, height) = decoder.dimensions();
    let frames = decoder.into_frames();
    let vec_frames;
    match frames.collect_frames() {
        Ok(result) => {
            vec_frames = result;
        }
        Err(_) => {
            return Err(CompressFail);
        }
    }

    let mut new_frames: Vec<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> = Vec::new();

    for frame in &vec_frames {
        let buffer = frame.buffer().to_owned();
        let rgba = image::ImageBuffer::from_raw(width, height, buffer).ok_or(CompressFail)?;
        let new_image = image::imageops::resize(&rgba, width / 2, height / 2, FilterType::Nearest);
        new_frames.push(new_image);
    }

    let origin_file_name = Path::new(&gif_path).file_stem().unwrap().to_str().ok_or(FileSystemFail)?;
    let compressed_gif_path = format!("{}/{}{}.gif", result_save_dir, origin_file_name, result_filename_suffix);
    let file_out = File::create(&compressed_gif_path).map_err(|_| FileSystemFail)?;

    let color_map = &[0u8, 0, 0, 255, 255, 255];
    let mut encoder = Encoder::new(file_out, width as u16 / 2, height as u16 / 2, color_map).map_err(|_| CompressFail)?;

    encoder.set_repeat(Repeat::Infinite).map_err(|_| CompressFail)?;

    for mut frame in new_frames {
        let (new_width, new_height) = (frame.width() as u16, frame.height() as u16);
        let new_frame = Frame::from_rgba(new_width, new_height, &mut *frame);
        encoder.write_frame(&new_frame).map_err(|_| CompressFail)?;
    }

    Ok(compressed_gif_path)
}