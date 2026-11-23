use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Mermaid,
    Svg,
    Html,
    Png,
    Jpeg,
    Webp,
}

impl Format {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Mermaid => "mermaid",
            Self::Svg => "svg",
            Self::Html => "html",
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Webp => "webp",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VisualInput {
    pub title: String,
    pub format: Format,
    /// Source text, or base64 bytes for PNG/JPEG/WebP. Prefer local_file in the MCP tool for image files.
    pub content: String,
    #[serde(default)]
    pub description: String,
    /// Omit to create a visual. Supply with expected_revision to replace one.
    pub id: Option<String>,
    pub expected_revision: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommentInput {
    pub visual: String,
    pub revision: i64,
    pub text: String,
}

#[derive(Debug)]
pub struct Error(pub StatusCode, pub String);
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn bad(message: &str) -> Self {
        Self(StatusCode::BAD_REQUEST, message.into())
    }
    pub fn missing() -> Self {
        Self(StatusCode::NOT_FOUND, "Not found in this workspace".into())
    }
}
impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        eprintln!("Database operation failed: {error}");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database operation failed".into(),
        )
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error":self.1}))).into_response()
    }
}
