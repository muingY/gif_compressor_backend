use std::path::Path;
use std::time::Duration;
use actix_web::dev::ResourcePath;
use actix_web::web;
use chrono::Utc;
use tokio::{time};
use crate::state::AppState;

pub async fn session_cleanup_task(app_state: web::Data<AppState>) {
    let interval = time::interval(Duration::from_secs(60 * 5)); // 5 min

    tokio::pin!(interval);

    loop {
        interval.tick().await;

        let mut sessions = app_state.sessions.lock().unwrap();
        let now = Utc::now();

        let mut need_cleanup: bool = false;

        for session in sessions.iter() {
            if (now - *session.1).num_seconds() > (60 * 60 * 2) {
                let path = format!("./gifs/{}", session.0);

                if Path::new(&*path.path()).exists() {
                    std::fs::remove_dir_all(&*path).unwrap();
                }

                need_cleanup = true;
            }
        }

        if need_cleanup {
            sessions.retain(|_, &mut v| (now - v).num_seconds() < (60 * 60 * 2));
        }
    }
}