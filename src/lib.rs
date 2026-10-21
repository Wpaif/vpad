pub mod dispositivo;
pub mod mapeamento;
pub mod protocolo;

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};
use rust_embed::RustEmbed;
use tokio::sync::mpsc::{self, UnboundedSender};

use protocolo::{Evento, decodificar};

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

type Saida = UnboundedSender<Evento>;

async fn rota_ws(ws: WebSocketUpgrade, State(saida): State<Saida>) -> Response {
    ws.on_upgrade(move |socket| conexao(socket, saida))
}

async fn conexao(mut ws: WebSocket, saida: Saida) {
    while let Some(Ok(msg)) = ws.recv().await {
        let Message::Binary(dados) = msg else { continue };
        match decodificar(&dados) {
            Some(Evento::Ping) => {
                if ws.send(Message::Binary(dados)).await.is_err() {
                    break;
                }
            }
            Some(evento) => {
                let _ = saida.send(evento);
            }
            None => {}
        }
    }
    // Soltar tudo evita botões "presos" se o celular some no meio de um toque.
    let _ = saida.send(Evento::SoltarTudo);
}

/// Servidor cujos eventos de controle são enviados para `saida`.
pub fn app_com(saida: Saida) -> Router {
    Router::new()
        .route("/ws", get(rota_ws))
        .with_state(saida)
        .fallback(get(estatico))
}

/// Servidor que descarta os eventos (só serve a página).
pub fn app() -> Router {
    app_com(mpsc::unbounded_channel().0)
}
