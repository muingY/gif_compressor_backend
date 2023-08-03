use std::fs::File;
use std::path::Path;
use actix_multipart::Multipart;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use actix_web::cookie::Cookie;
use gif::{Encoder, Frame, Repeat};
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, ImageDecoder};
use image::imageops::FilterType;
use serde_json::json;
use crate::component::{payload_save, PayloadFileFailType, PayloadSaveErrType};
use crate::state::AppState;

pub async fn compress_gif(app_state: web::Data<AppState>, payload: Multipart, req: HttpRequest) -> impl Responder {
    let session_id = app_state.new_session();
    let save_path = format!("./gifs/{}", session_id);

    let mut success_raw_file_path_list: Vec<String> = Vec::new();
    let mut fail_file_list: Vec<(String, PayloadFileFailType)> = Vec::new();
    match payload_save(
        payload,
        req,
        save_path.clone(),
        vec![mime::IMAGE_GIF],
        200000_000,
        10
    ).await {
        Ok((success_file_paths, fail_files)) => {
            success_raw_file_path_list = success_file_paths;
            fail_file_list = fail_files;
        },
        Err(error_type) => {
            match error_type {
                PayloadSaveErrType::SizeLimitExceed | PayloadSaveErrType::FileNotAttached=> {
                    return HttpResponse::BadRequest()
                        .json(json!({
                            "error": error_type as i32
                        }))
                }
                PayloadSaveErrType::FileSystemFail | PayloadSaveErrType::ServerErr => {

                }
            }
        },
    }

    // Compress
    let mut compress_success_count: u8 = 0;

    for raw_file_path in success_raw_file_path_list {
        let file_in = File::open(raw_file_path.clone()).unwrap();
        let decoder = GifDecoder::new(file_in).unwrap();
        let (width, height) = decoder.dimensions();
        let frames = decoder.into_frames();
        let frames = frames.collect_frames().unwrap();

        let mut new_frames: Vec<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> = Vec::new();

        for frame in &frames {
            let buffer = frame.buffer().to_owned();
            let rgba = image::ImageBuffer::from_raw(width, height, buffer).unwrap();
            let new_image = image::imageops::resize(&rgba, width / 2, height / 2, FilterType::Nearest);
            new_frames.push(new_image);
        }

        let file_name = Path::new(&raw_file_path).file_stem().unwrap().to_str().unwrap();
        let compressed_gif_path = format!("{}/{}-{}.gif", &save_path, file_name, "compressed");
        let file_out = File::create(&compressed_gif_path).unwrap();

        let color_map = &[0u8, 0, 0, 255, 255, 255]; // Black and white
        let mut encoder = Encoder::new(file_out, width as u16 / 2, height as u16 / 2, color_map).unwrap();

        encoder.set_repeat(Repeat::Infinite).unwrap();

        for mut frame in new_frames {
            let (new_width, new_height) = (frame.width() as u16, frame.height() as u16);
            let new_frame = Frame::from_rgba(new_width, new_height, &mut *frame);
            encoder.write_frame(&new_frame).unwrap();
        }

        compress_success_count += 1;
    }

    // Result
    HttpResponse::Ok()
        .cookie(Cookie::build("session", session_id).finish())
        .json(json!({
            "success": compress_success_count,
            "fail_list": fail_file_list.len()
        }))
}