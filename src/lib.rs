mod api;
pub mod acesso;
pub mod armazem;
pub mod convite;
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
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{
        mpsc::{self, UnboundedSender},
        watch,
    },
    time::{Instant, sleep_until},
};

use acesso::{Acesso, Entrada};
use armazem::ArmazemMapa;
use protocolo::{Evento, Recusa, codificar_perfil, codificar_recusa, decodificar};


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
    acesso: Arc<Acesso>,
}

/// Parâmetros da URL do WebSocket: `/ws?t=<token>&id=<aparelho>`.
#[derive(Deserialize)]
struct ParametrosWs {
    t: Option<String>,
    id: Option<String>,
}

/// O id identifica o aparelho (o celular o gera e guarda); só aceita texto simples e curto.
fn id_valido(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

async fn rota_ws(
    ws: WebSocketUpgrade,
    Query(parametros): Query<ParametrosWs>,
    State(estado): State<Estado>,
) -> Response {
    ws.on_upgrade(move |socket| conexao(socket, estado, parametros))
}

async fn avisar_perfil(ws: &mut WebSocket, indice: u8) -> Result<(), axum::Error> {
    ws.send(Message::Binary(codificar_perfil(indice).to_vec().into())).await
}

/// Avisa o celular do motivo e encerra a conexão.
async fn recusar(mut ws: WebSocket, motivo: Recusa) {
    let _ = ws.send(Message::Binary(codificar_recusa(motivo).to_vec().into())).await;
    let _ = ws.send(Message::Close(None)).await;
}

async fn conexao(ws: WebSocket, estado: Estado, parametros: ParametrosWs) {
    let Estado { saida, mut perfil, acesso, .. } = estado;

    let token = parametros.t.unwrap_or_default();
    let cliente = parametros.id.unwrap_or_default();
    if !acesso.autorizado(&token) || !id_valido(&cliente) {
        return recusar(ws, Recusa::NaoAutorizado).await;
    }
    let mut vaga = match acesso.entrar(&cliente) {
        Entrada::Dono(vaga) => vaga,
        Entrada::Ocupado => return recusar(ws, Recusa::Ocupado).await,
    };
    let mut ws = ws;

    // Tomou o lugar de outra conexão do mesmo aparelho: começa sem botões presos da antiga.
    if vaga.substituiu {
        let _ = saida.send(Evento::SoltarTudo);
    }

    let atual = *perfil.borrow_and_update();
    if avisar_perfil(&mut ws, atual).await.is_ok() {
        let inatividade = acesso.inatividade();
        let mut limite = Instant::now() + inatividade;
        loop {
            tokio::select! {
                biased;
                // outra conexão do mesmo aparelho assumiu
                _ = &mut vaga.derrubada => break,
                // o celular parou de dar sinal (o ping dele chega todo segundo)
                _ = sleep_until(limite) => break,
                msg = ws.recv() => {
                    let Some(Ok(msg)) = msg else { break };
                    limite = Instant::now() + inatividade;
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
    // Soltar tudo evita botões "presos" se o celular some no meio de um toque. Só quem ainda
    // era o dono faz isso: uma conexão substituída não pode soltar os botões da nova.
    if vaga.liberar() {
        let _ = saida.send(Evento::SoltarTudo);
    }
}

/// Servidor cujos eventos de controle vão para `saida`; `perfil` informa o perfil ativo,
/// `mapa` guarda as teclas do modo teclado e `acesso` decide quem pode controlar.
pub fn app_com(
    saida: Saida,
    perfil: watch::Receiver<u8>,
    mapa: Arc<ArmazemMapa>,
    acesso: Arc<Acesso>,
) -> Router {
    Router::new()
        .route("/ws", get(rota_ws))
        .route("/api/teclado", get(api::obter).put(api::definir))
        .with_state(Estado { saida, perfil, mapa, acesso })
        .fallback(get(estatico))
}

/// Servidor que descarta os eventos (só serve a página). Sem token configurado, nenhuma
/// conexão de controle nem chamada à API é aceita.
pub fn app() -> Router {
    app_com(
        mpsc::unbounded_channel().0,
        watch::channel(0).1,
        Arc::new(ArmazemMapa::em_memoria()),
        Acesso::novo("", Duration::from_secs(6)),
    )
}
