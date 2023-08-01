use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde_json::json;
use crate::state::AppState;

pub async fn compress_result_download(app_state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
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