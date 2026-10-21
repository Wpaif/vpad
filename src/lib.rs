use axum::{
    Router,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};
use rust_embed::RustEmbed;

/// Páginas geradas pelo Vite. Em debug lê do disco; em release vai dentro do binário.
#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Web;

async fn estatico(uri: Uri) -> Response {
    let caminho = match uri.path().trim_start_matches('/') {
        "" => "index.html",
        p => p,
    };
    match Web::get(caminho) {
        Some(arquivo) => {
            let tipo = mime_guess::from_path(caminho).first_or_octet_stream();
            let tipo = if tipo.type_() == "text" {
                format!("{tipo}; charset=utf-8")
            } else {
                tipo.to_string()
            };
            ([(header::CONTENT_TYPE, tipo)], arquivo.data.into_owned()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub fn app() -> Router {
    Router::new().fallback(get(estatico))
}
