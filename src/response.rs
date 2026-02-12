use axum::response::Json;
use serde::Serialize;
use std::time::Instant;

#[derive(Default, Serialize)]
pub struct Response {
    message: String,
    elapsed: u128,
}

impl Response {
    pub fn new(message: String, timer: Option<&Instant>) -> Json<Self> {
        Json(Self {
            message,
            elapsed: timer.map(|t| t.elapsed().as_millis()).unwrap_or_default(),
        })
    }
}
