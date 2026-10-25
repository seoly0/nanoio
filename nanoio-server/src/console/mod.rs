use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    Router,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../nanoio-web/dist"]
struct WebAssets;

pub fn create_console_router() -> Router {
    Router::new().fallback(static_handler)
}

async fn static_handler(uri: Uri) -> Response {
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }

    match WebAssets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref())],
                Body::from(content.data),
            )
                .into_response()
        }
        None => {
            // SPA fallback: index.html
            match WebAssets::get("index.html") {
                Some(content) => (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/html")],
                    Body::from(content.data),
                )
                    .into_response(),
                None => (
                    StatusCode::NOT_FOUND,
                    "Web console assets not bundled",
                )
                    .into_response(),
            }
        }
    }
}
