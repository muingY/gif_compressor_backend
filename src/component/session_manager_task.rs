use std::path::Path;
use std::time::Duration;
use actix_web::dev::ResourcePath;
use actix_web::web;
use chrono::Utc;
use tokio::{time};
use crate::state::AppState;

pub async fn session_cleanup_task(app_state: web::Data<AppState>) {
    let interval = time::interval(Duration::from_secs(10)); // 5 min

    tokio::pin!(interval);

    loop {
        interval.tick().await;

        let mut sessions = app_state.sessions.lock().unwrap();
        let now = Utc::now();
        let mut to_remove: Vec<String> = Vec::new();

        for session in sessions.iter() {
            if (now - *session.1).num_seconds() > (30) {
                let path = format!("./gifs/{}", session.0);

                if Path::new(&*path.path()).exists() {
                    std::fs::remove_dir_all(&*path).unwrap();
                }

                to_remove.push(session.0.to_string());
            }
        }

        for remove_session in to_remove {
            sessions.remove(remove_session.as_str());
        }
    }
}