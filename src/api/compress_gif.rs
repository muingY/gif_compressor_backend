use std::{fs, path};
use actix_multipart::Multipart;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use actix_web::cookie::Cookie;
use serde_json::json;
use crate::component::{compress, CompressErrType, payload_save, PayloadFileFailType, PayloadSaveErrType};
use crate::state::AppState;

pub async fn compress_gif(app_state: web::Data<AppState>, payload: Multipart, req: HttpRequest) -> impl Responder {
    let session_id = app_state.new_session();
    let save_path = format!("./gifs/{}", session_id);

    // Uploaded files save process.
    let success_raw_file_paths: Vec<String>;
    let upload_fail_files: Vec<(String, PayloadFileFailType)>;
    match payload_save(
        payload,
        req,
        save_path.clone(),
        vec![mime::IMAGE_GIF],
        200000_000,
        10
    ).await {
        Ok((success_file_paths, fail_files)) => {
            success_raw_file_paths = success_file_paths;
            upload_fail_files = fail_files;
        },
        Err(error_type) => {
            return match error_type {
                PayloadSaveErrType::SizeLimitExceed | PayloadSaveErrType::FileNotAttached => {
                    HttpResponse::BadRequest()
                        .json(json!({
                            "error": error_type as i32
                        }))
                }
                PayloadSaveErrType::FileSystemFail | PayloadSaveErrType::ServerErr => {
                    HttpResponse::InternalServerError()
                        .json(json!({
                            "error": error_type as i32
                        }))
                }
            }
        },
    }

    // Compress
    let mut compressed_files: Vec<(String, String)> = vec!();
    let mut compress_fail_file_list: Vec<(String, CompressErrType)> = vec!();

    for raw_file_path in success_raw_file_paths {
        match compress(
            raw_file_path.clone(),
            save_path.clone(),
            "-compressed".to_string()
        ).await {
            Ok(compressed_file_path) => {
                compressed_files.push((raw_file_path, compressed_file_path));
            }
            Err(err) => {
                compress_fail_file_list.push((raw_file_path.to_string(), err));
            }
        }
    }
    if compressed_files.len() == 0 {
        return HttpResponse::InternalServerError()
            .json(json!({
                "error": 10
            }));
    }

    // TODO: Write compress result analysis in json.
    let mut compress_result: Vec<(String, u64, u64, f32)> = vec!();
    for (raw_file, compressed_file) in compressed_files.clone() {
        let raw_size = fs::metadata(raw_file.clone()).unwrap().len();
        let compressed_size = fs::metadata(compressed_file).unwrap().len();
        let compress_rate = (1.0 - (compressed_size.clone() as f32 / raw_size.clone() as f32)) * 100.0;
        compress_result.push((
            path::Path::new(raw_file.as_str()).file_name().unwrap().to_str().unwrap().to_string(),
            raw_size,
            compressed_size,
            compress_rate
            ));
    }
    let compress_result_to_json = |compress_result: Vec<(String, u64, u64, f32)>| {
        let json_array: Vec<_> = compress_result.iter().map(|(filename, raw_size, compressed_size, compress_rate)| {
            json!({
            "filename": filename,
            "raw_size": raw_size,
            "compressed_size": compressed_size,
            "compress_rate": compress_rate
        })
        }).collect();
        json_array
    };

    // Result
    // TODO: Add compress result analysis data to response.
    HttpResponse::Ok()
        .cookie(Cookie::build("session", session_id).finish())
        .json(json!({
            "success": compressed_files.len(),
            "success_detail": compress_result_to_json(compress_result),
            "fail": upload_fail_files.len()
            // "fail_detail": compress_fail_file_list,
        }))
}