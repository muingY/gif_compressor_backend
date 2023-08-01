use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde_json::json;
use crate::state::AppState;

pub async fn check_session(app_state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
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