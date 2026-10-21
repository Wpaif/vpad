mod api;
pub mod armazem;
pub mod dispositivo;
pub mod mapa;
pub mod mapeamento;
pub mod perfil;
pub mod protocolo;
pub mod servico;
pub mod teclado;
pub mod teclas;

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
use std::sync::Arc;
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    watch,
};

use armazem::ArmazemMapa;
use protocolo::{Evento, codificar_perfil, decodificar};


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

#[derive(Clone)]
struct Estado {
    saida: Saida,
    /// Índice do perfil ativo, publicado pelo serviço dono do controle virtual.
    perfil: watch::Receiver<u8>,
    mapa: Arc<ArmazemMapa>,
}

async fn rota_ws(ws: WebSocketUpgrade, State(estado): State<Estado>) -> Response {
    ws.on_upgrade(move |socket| conexao(socket, estado))
}

async fn avisar_perfil(ws: &mut WebSocket, indice: u8) -> Result<(), axum::Error> {
    ws.send(Message::Binary(codificar_perfil(indice).to_vec().into())).await
}

async fn conexao(mut ws: WebSocket, Estado { saida, mut perfil, .. }: Estado) {
    let atual = *perfil.borrow_and_update();
    if avisar_perfil(&mut ws, atual).await.is_ok() {
        loop {
            tokio::select! {
                msg = ws.recv() => {
                    let Some(Ok(msg)) = msg else { break };
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
                Ok(()) = perfil.changed() => {
                    let novo = *perfil.borrow_and_update();
                    if avisar_perfil(&mut ws, novo).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
    // Soltar tudo evita botões "presos" se o celular some no meio de um toque.
    let _ = saida.send(Evento::SoltarTudo);
}

/// Servidor cujos eventos de controle vão para `saida`; `perfil` informa o perfil ativo e
/// `mapa` guarda as teclas do modo teclado.
pub fn app_com(saida: Saida, perfil: watch::Receiver<u8>, mapa: Arc<ArmazemMapa>) -> Router {
    Router::new()
        .route("/ws", get(rota_ws))
        .route("/api/teclado", get(api::obter).put(api::definir))
        .with_state(Estado { saida, perfil, mapa })
        .fallback(get(estatico))
}

/// Servidor que descarta os eventos (só serve a página).
pub fn app() -> Router {
    app_com(mpsc::unbounded_channel().0, watch::channel(0).1, Arc::new(ArmazemMapa::em_memoria()))
}
