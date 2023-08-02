use std::fs;
use std::fs::File;
use std::io::{ErrorKind, Write};
use actix_files::NamedFile;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use tokio::io::AsyncReadExt;
use crate::state::AppState;

pub async fn compress_result_download(app_state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let sessions = app_state.sessions.lock().unwrap();

    let session: String;
    match req.cookie("session") {
        Some(cookie) => { session = cookie.value().to_string() },
        None => {
            return HttpResponse::BadRequest().finish();
        }
    }

    if !sessions.contains_key(&*session) {
        return HttpResponse::BadRequest().finish();
    }

    let path = format!("./gifs/{}", session);

    let mut compressed_files = Vec::new();

    match fs::read_dir(&path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.ends_with("-compressed.gif") {
                            compressed_files.push(entry.path());
                        }
                    }
                }
            }
        },
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            return HttpResponse::NotFound().finish();
        },
        Err(_) => {
            return HttpResponse::InternalServerError().finish();
        },
    }

    match compressed_files.len() {
        0 => HttpResponse::NotFound().finish(),
        1 => match NamedFile::open(&compressed_files[0]) {
            Ok(file) => file.into_response(&req),
            Err(_) => HttpResponse::InternalServerError().finish(),
        },
        _ => {
            let zip_file_path = format!("{}.zip", path);
            let zip_file = File::create(&zip_file_path).unwrap();
            let mut zip = zip::ZipWriter::new(zip_file);
            let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
                .unix_permissions(0o755);

            for path in compressed_files {
                // let file_name = path.file_name().unwrap();
                // zip.start_file(file_name.to_string_lossy(), options).unwrap();
                // let mut file = File::open(&path).unwrap();
                // file.copy_into(&mut zip).await.unwrap();
                let file_name = path.file_name().unwrap();
                zip.start_file(file_name.to_string_lossy(), options).unwrap();
                let mut file = tokio::fs::File::open(&path).await.unwrap();
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).await.unwrap();
                zip.write_all(&buffer).unwrap();
            }
            zip.finish().unwrap();
            let named_file = NamedFile::open(zip_file_path).unwrap();

            named_file.into_response(&req)
        },
    }
}