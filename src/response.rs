use std::time::Instant;

use serde::Serialize;

#[derive(Default, Serialize)]
pub struct Response {
    message: String,
    elapsed: u128,
}

impl Response {
    pub fn new(message: String, timer: &Instant) -> Self {
        Self {
            message,
            elapsed: timer.elapsed().as_millis(),
        }
    }
}
