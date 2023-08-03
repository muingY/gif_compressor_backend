mod state;
mod api;
mod component;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex};
use actix_web::{web, App, HttpServer};
use tokio::fs;
use crate::api::{check_session, compress_gif, compress_result_download};
use crate::component::session_cleanup_task;
use crate::state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if Path::new("./gifs").exists() {
        fs::remove_dir_all("./gifs").await
            .expect("Error: Initialize error. ./gifs dir remove fail.");
    }
    fs::create_dir("./gifs")
        .await.expect("Error: Initialize error. ./gifs dir create fail.");

    let app_state = web::Data::new(AppState {
        sessions: Mutex::new(HashMap::new()),
    });

    tokio::spawn(session_cleanup_task(app_state.clone()));

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/gif-compressor")
                            .route("/check-session", web::get().to(check_session))
                            .route("/compress", web::post().to(compress_gif))
                            .route("/download", web::get().to(compress_result_download))
                    )
            )
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}