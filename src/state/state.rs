use std::collections::HashMap;
use std::sync::Mutex;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct AppState {
    pub sessions: Mutex<HashMap<String, DateTime<Utc>>>,
}

impl AppState {
    pub fn new_session(&self) -> String {
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