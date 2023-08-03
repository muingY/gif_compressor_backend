use std::fs;
use std::io::Write;
use std::path::Path;
use actix_multipart::Multipart;
use actix_web::dev::ResourcePath;
use actix_web::http::header::CONTENT_LENGTH;
use actix_web::HttpRequest;
use futures_util::TryStreamExt;
use mime::Mime;
use crate::component::payload_save::PayloadFileFailType::{FileErr, TypeMismatch};
use crate::component::payload_save::PayloadSaveErrType::{FileNotAttached, FileSystemFail, ServerErr, SizeLimitExceed};

pub enum PayloadSaveErrType {
    SizeLimitExceed = 0,
    FileNotAttached = 1,
    FileSystemFail = 3,
    ServerErr = 4,
}

pub enum PayloadFileFailType {
    TypeMismatch = 0,
    FileErr = 1,
}

pub async fn payload_save(
    mut payload: Multipart,
    req: HttpRequest,
    save_path: String,
    allow_filetypes: Vec<Mime>,
    max_file_size: usize,
    max_file_count: usize
) -> Result<(Vec<String>, Vec<(String, PayloadFileFailType)>), PayloadSaveErrType> {
    let content_length: usize = match req.headers().get(CONTENT_LENGTH) {
        Some(header_value) => match header_value.to_str() {
            Ok(s) => match s.parse::<usize>() {
                Ok(n) => n,
                Err(_) => 0,
            },
            Err(_) => 0,
        },
        None => 0,
    };

    if max_file_size < content_length {
        return Err(SizeLimitExceed);
    }

    let mut success_file_paths: Vec<String> = Vec::with_capacity(max_file_count);
    let mut fail_files: Vec<(String, PayloadFileFailType)> = Vec::new();

    if !Path::new(&*save_path.path()).exists() {
        fs::create_dir(&*save_path).map_err(|_| FileSystemFail)?
    }

    let mut current_count: usize = 0;

    loop {
        if current_count >= max_file_count {
            break;
        }
        if let Ok(Some(mut field)) = payload.try_next().await {
            let filetype = field.content_type();
            if filetype.is_none() || !allow_filetypes.contains(filetype.ok_or(ServerErr)?) {
                fail_files.push((
                    field.content_disposition().get_filename().ok_or(ServerErr)?.to_string(),
                    TypeMismatch
                ));
                continue;
            }

            let filename: &str;
            match field.content_disposition().get_filename() {
                None => {
                    fail_files.push((
                        field.content_disposition().get_filename().ok_or(ServerErr)?.to_string(),
                        FileErr
                    ));
                    continue;
                }
                Some(result) => {
                    filename = result;
                }
            }

            let destination: String = format!(
                "{}/{}",
                save_path,
                filename
            );

            let mut saved_file: fs::File;
            match fs::File::create(&destination) {
                Ok(result) => { saved_file = result; }
                Err(_) => {
                    fail_files.push((
                        field.content_disposition().get_filename().ok_or(ServerErr)?.to_string(),
                        FileErr
                    ));
                    continue;
                }
            }
            while let Ok(Some(chunk)) = field.try_next().await {
                let _ = saved_file.write_all(&chunk);
            }
            success_file_paths.push(destination);
        } else {
            break;
        }
        current_count += 1;
    }

    if current_count == 0 {
        if Path::new(&*save_path.path()).exists() {
            fs::remove_dir_all(&*save_path).map_err(|_| FileSystemFail)?;
        }
        return Err(FileNotAttached);
    }

    Ok((success_file_paths, fail_files))
}