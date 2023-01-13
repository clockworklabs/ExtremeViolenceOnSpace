use hyper::http;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("JSON Error: `{0}`")]
    Json(#[from] serde_json::Error),
    #[error("IO Error: `{0}`")]
    Io(#[from] std::io::Error),
    #[error("Hyper Error: `{0}`")]
    Hyper(#[from] hyper::http::Error),
    #[error("Tungstenite Error: `{0}`")]
    Tungstenite(#[from] tungstenite::Error),
    #[error("InvalidUri Error: `{0}`")]
    InvalidUri(#[from] http::uri::InvalidUri),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
