use axum::response::Json;
use serde::Serialize;
use std::time::Instant;

#[derive(Default, Serialize)]
pub struct Response {
    message: String,
    elapsed: u128,
}

impl Response {
    pub fn new(message: String, timer: &Instant) -> Json<Self> {
        Json(Self {
            message,
            elapsed: timer.elapsed().as_millis(),
        })
    }
}
