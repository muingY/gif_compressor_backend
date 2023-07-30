use std::collections::HashMap;
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

async fn upload_gif(app_state: web::Data<AppState>, mut payload: Multipart, req: HttpRequest) -> impl Responder {
    let max_file_size: usize = 10000_000; // kb
    let max_file_count = 10;
    let legal_filetype = mime::IMAGE_GIF;

    let content_length: usize = match req.headers().get(http::header::CONTENT_LENGTH) {
        Some(header_value) => header_value.to_str().unwrap_or("0").parse().unwrap(),
        None => "0".parse().unwrap()
    };
    if max_file_size < content_length {
        return HttpResponse::BadRequest().json(json!({"error": "File size limit exceeded."}));
    }

    let session_id = app_state.new_session();
    let mut success_file_count = 0;
    let mut fail_file_list: Vec<String> = Vec::new();
    let path = format!("./upload_gif/{}", session_id);

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
                "{}/{}-{}",
                path,
                Uuid::new_v4(),
                field.content_disposition().get_filename().unwrap()
            );
            success_file_count += 1;

            let mut saved_file: fs::File = fs::File::create(&destination).await.unwrap();
            while let Ok(Some(chunk)) = field.try_next().await {
                let _ = saved_file.write_all(&chunk).await.unwrap();
            }
        } else {
            break;
        }
        current_count += 1;
    }

    HttpResponse::Ok()
        .cookie(Cookie::build("session", session_id).finish())
        .json(json!({
            "success": success_file_count,
            "fail_list": fail_file_list
        }))
}

async fn compress() -> impl Responder {
    HttpResponse::Ok().finish()
}

async fn result_download() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if !Path::new("./upload_gif").exists() {
        fs::create_dir("./upload_gif").await?;
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
                            .route("/upload", web::post().to(upload_gif))
                            .route("/compress", web::get().to(compress))
                            .route("/download", web::get().to(result_download))
                    )
            )
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}