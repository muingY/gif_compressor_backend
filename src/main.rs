use std::collections::HashMap;
use std::fmt::format;
use std::fs::File;
use std::path::Path;
use std::sync::{ Mutex };
use actix_web::{ web, http, App, HttpServer, Responder, HttpResponse, HttpRequest };
use actix_multipart::{ Multipart };
use actix_web::cookie::Cookie;
use actix_web::dev::ResourcePath;
use futures_util::{ TryStreamExt };
use mime;
use mime::Mime;
use uuid::Uuid;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use serde_json::json;
use chrono::{ Utc, DateTime };
use gif::{Encoder, Frame, Repeat};
use image::{AnimationDecoder, ImageDecoder};
use image::codecs::gif::{GifDecoder};
use image::imageops::{FilterType};

struct AppState {
    sessions: Mutex<HashMap<String, DateTime<Utc>>>,
}

impl AppState {
    fn new_session(&self) -> String {
        let mut sessions = self.sessions.lock().unwrap();

        let mut session_id = Uuid::new_v4().to_string();
        while sessions.contains_key(&*session_id) {
            session_id = Uuid::new_v4().to_string();
        }
        let new_created_time: DateTime<Utc> = Utc::now();

        sessions.insert(session_id.clone(), new_created_time);

        session_id
    }
}

async fn check_session(app_state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let sessions = app_state.sessions.lock().unwrap();

    let check_session: String;
    match req.cookie("session") {
        Some(cookie) => { check_session = cookie.to_string(); },
        None => {
            return HttpResponse::Unauthorized().finish();
        }
    }

    if sessions.contains_key(check_session.as_str()) {
        return HttpResponse::Ok()
            .json(json!({
                "session_exist": true,
                // "compress_result": ...
            }));
    }

    HttpResponse::Unauthorized().finish()
}

async fn compress_gif(app_state: web::Data<AppState>, mut payload: Multipart, req: HttpRequest) -> impl Responder {
    // File upload
    let max_file_size: usize = 10000_000; // kb
    let max_file_count = 10;
    let legal_filetype = mime::IMAGE_GIF;

    let content_length: usize = match req.headers().get(http::header::CONTENT_LENGTH) {
        Some(header_value) => header_value.to_str().unwrap_or("0").parse().unwrap(),
        None => "0".parse().unwrap()
    };
    if max_file_size < content_length {
        return HttpResponse::BadRequest().json(json!({
            "error": "File size limit exceeded.",
            "errno": 1
        }));
    }

    let session_id = app_state.new_session();
    let mut success_raw_file_path_list: Vec<String> = Vec::with_capacity(max_file_count);
    let mut fail_file_list: Vec<String> = Vec::new();
    let path = format!("./gifs/{}", session_id);

    if !Path::new(&*path.path()).exists() {
        fs::create_dir(&*path).await.unwrap();
    }

    let mut current_count: usize = 0;

    loop {
        if current_count >= max_file_count {
            break;
        }
        if let Ok(Some(mut field)) = payload.try_next().await {
            let filetype: Option<&Mime> = field.content_type();
            if filetype.is_none() {
                continue;
            }
            if legal_filetype != *filetype.unwrap() {
                fail_file_list.push(field.content_disposition().get_filename().unwrap().to_string());
                continue;
            }

            let destination: String = format!(
                "{}/{}",
                path,
                field.content_disposition().get_filename().unwrap()
            );

            let mut saved_file: fs::File = fs::File::create(&destination).await.unwrap();
            while let Ok(Some(chunk)) = field.try_next().await {
                let _ = saved_file.write_all(&chunk).await.unwrap();
            }
            success_raw_file_path_list.push(destination);
        } else {
            break;
        }
        current_count += 1;
    }

    if current_count == 0 {
        if Path::new(&*path.path()).exists() {
            fs::remove_dir_all(&*path).await.unwrap();
        }
        return HttpResponse::BadRequest().json(json!({
            "error": "File not attached.",
            "errno": 2
        }));
    }

    let mut compress_success_count: u8 = 0;

    // Compress
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
        let compressed_gif_path = format!("{}/{}-{}.gif", &path, file_name, "compressed");
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
            "fail_list": fail_file_list
        }))
}

async fn result_download(app_state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let sessions = app_state.sessions.lock().unwrap();

    let session: String;
    match req.cookie("session") {
        Some(cookie) => { session = cookie.to_string(); },
        None => {
            return HttpResponse::BadRequest().finish();
        }
    }

    if !sessions.contains_key(&*session) {
        return HttpResponse::BadRequest().finish();
    }

    let path = format!("./gifs/{}", session);
    // ...

    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if !Path::new("../gifs").exists() {
        fs::create_dir("../gifs").await?;
    }

    let app_state = web::Data::new(AppState {
        sessions: Mutex::new(HashMap::new()),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/gif-compressor")
                            .route("/check-session", web::get().to(check_session))
                            .route("/compress", web::post().to(compress_gif))
                            .route("/download", web::get().to(result_download))
                    )
            )
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}