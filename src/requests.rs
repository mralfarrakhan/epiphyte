use axum::{
    body::{self, Body},
    extract::FromRequest,
    http::{Request, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Text {
    pub message: String,
}
pub enum MultiPayload {
    Text(Text),
    Signal,
}

#[derive(Debug)]
pub struct MultiPayloadRejection(String);

impl<S> From<S> for MultiPayloadRejection
where
    S: ToString,
{
    fn from(value: S) -> Self {
        MultiPayloadRejection(value.to_string())
    }
}

impl IntoResponse for MultiPayloadRejection {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid or unreadable body: {}", self.0),
        )
            .into_response()
    }
}

impl<S> FromRequest<S> for MultiPayload
where
    S: Send + Sync,
{
    type Rejection = MultiPayloadRejection;

    async fn from_request(req: Request<Body>, _state: &S) -> Result<Self, Self::Rejection> {
        let bytes = body::to_bytes(req.into_body(), 1025 * 1024)
            .await
            .map_err(MultiPayloadRejection::from)?;

        if bytes.is_empty() {
            return Ok(Self::Signal);
        }

        if let Ok(v) = serde_json::from_slice::<Text>(&bytes) {
            return Ok(Self::Text(v));
        }

        Err(MultiPayloadRejection::from(
            "Request payload didn't match any known format",
        ))
    }
}
