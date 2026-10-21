mod api;
pub mod acesso;
pub mod aprovacao;
pub mod alerta;
pub mod armazem;
pub mod cli;
pub mod configuracao;
pub mod console;
pub mod convite;
pub mod dispositivo;
pub mod dispositivos;
pub mod https;
pub mod mapa;
pub mod mobileconfig;
pub mod pareamento;
pub mod mapeamento;
pub mod perfil;
pub mod protocolo;
pub mod servico;
pub mod tls;
pub mod totp;
pub mod teclado;
pub mod teclas;

use axum::{
    Router,
    extract::{
        ConnectInfo, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::{net::{IpAddr, SocketAddr}, sync::Arc, time::Duration};
use tokio::{
    sync::{
        mpsc::{self, UnboundedSender},
        watch,
    },
    time::{Instant, sleep_until},
};

use acesso::Acesso;
use aprovacao::{Decisao, Ocupado, Pedido, Tipo as Tipo_};
use dispositivos::Dispositivos;
use pareamento::Pareamento;
use armazem::ArmazemMapa;
use perfil::{PERFIS, Tipo};
use protocolo::{AvisoDePerfil, Evento, Recusa, codificar_aviso_de_perfil, codificar_perfil, codificar_recusa, decodificar};


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

/// Parâmetros da URL do WebSocket: `/ws?t=<token do aparelho>`.
#[derive(Deserialize)]
struct ParametrosWs {
    t: Option<String>,
}

async fn rota_ws(
    ws: WebSocketUpgrade,
    Query(parametros): Query<ParametrosWs>,
    State(estado): State<Estado>,
    partes: axum::http::request::Parts,
) -> Response {
    let origem = partes.extensions.get::<ConnectInfo<SocketAddr>>().map(|c| c.0.ip());
    ws.on_upgrade(move |socket| conexao(socket, estado, parametros, origem))
}

async fn avisar_perfil(ws: &mut WebSocket, indice: u8) -> Result<(), axum::Error> {
    ws.send(Message::Binary(codificar_perfil(indice).to_vec().into())).await
}

async fn enviar(ws: &mut WebSocket, mensagem: impl Into<Vec<u8>>) -> Result<(), axum::Error> {
    ws.send(Message::Binary(mensagem.into().into())).await
}

/// Avisa o celular de que não tem acesso e encerra a conexão.
async fn recusar(ws: &mut WebSocket) {
    let _ = enviar(ws, codificar_recusa(Recusa::NaoAutorizado)).await;
    let _ = ws.send(Message::Close(None)).await;
}

/// O perfil ativo agora é o de teclado (que digita no PC)?
fn modo_teclado_ativo(perfil: &watch::Receiver<u8>) -> bool {
    PERFIS.get(*perfil.borrow() as usize).is_some_and(|p| p.tipo == Tipo::Teclado)
}

fn e_perfil_de_teclado(indice: u8) -> bool {
    PERFIS.get(indice as usize).is_some_and(|p| p.tipo == Tipo::Teclado)
}

/// Pedido de um aparelho para ativar o modo teclado. Sem permissão, recusa; com permissão,
/// quem está no PC precisa confirmar. A espera roda à parte para a conexão seguir atendendo.
async fn pedir_teclado(
    ws: &mut WebSocket,
    estado: &Estado,
    token: &str,
    indice: u8,
    origem: Option<IpAddr>,
    retorno: &UnboundedSender<Vec<u8>>,
    ativacao: &mut Option<tokio::task::JoinHandle<()>>,
) -> Result<(), axum::Error> {
    let acesso = &estado.acesso;
    let Some(aparelho) = acesso.dispositivos().identificar(token) else { return Ok(()) };
    if !aparelho.teclado {
        return enviar(ws, codificar_aviso_de_perfil(AvisoDePerfil::SemPermissao)).await;
    }
    if *estado.perfil.borrow() == indice {
        let _ = estado.saida.send(Evento::TrocarPerfil(indice)); // já está no modo teclado
        return Ok(());
    }
    if ativacao.as_ref().is_some_and(|tarefa| !tarefa.is_finished()) {
        return enviar(ws, codificar_aviso_de_perfil(AvisoDePerfil::Aguardando)).await;
    }
    if acesso.aprovacoes().pendente().is_some() {
        return enviar(ws, codificar_aviso_de_perfil(AvisoDePerfil::Ocupado)).await;
    }

    enviar(ws, codificar_aviso_de_perfil(AvisoDePerfil::Aguardando)).await?;
    let aprovacoes = acesso.aprovacoes().clone();
    let prazo = acesso.prazo_da_aprovacao();
    let saida = estado.saida.clone();
    let retorno = retorno.clone();
    *ativacao = Some(tokio::spawn(async move {
        let pedido = Pedido { id: 0, tipo: Tipo_::AtivarTeclado { nome: aparelho.nome }, origem };
        let aviso = match aprovacoes.solicitar(pedido, prazo).await {
            Ok(Decisao::Aprovado { .. }) => {
                let _ = saida.send(Evento::TrocarPerfil(indice));
                return;
            }
            Ok(Decisao::Negado) => AvisoDePerfil::Negado,
            Ok(Decisao::Expirou) => AvisoDePerfil::Expirou,
            Err(Ocupado) => AvisoDePerfil::Ocupado,
        };
        let _ = retorno.send(codificar_aviso_de_perfil(aviso).to_vec());
    }));
    Ok(())
}

async fn conexao(mut ws: WebSocket, estado: Estado, parametros: ParametrosWs, origem: Option<IpAddr>) {
    let token = parametros.t.unwrap_or_default();
    if !estado.acesso.autorizado(&token) {
        return recusar(&mut ws).await;
    }
    let saida = estado.saida.clone();
    let acesso = estado.acesso.clone();
    let mut perfil = estado.perfil.clone();

    // respostas do pedido de ativação do teclado, que espera o "sim" no PC em outra tarefa
    let (retorno_tx, mut retorno_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let mut ativacao: Option<tokio::task::JoinHandle<()>> = None;

    let atual = *perfil.borrow_and_update();
    if avisar_perfil(&mut ws, atual).await.is_ok() {
        let inatividade = acesso.inatividade();
        let mut limite = Instant::now() + inatividade;
        loop {
            tokio::select! {
                biased;
                // o celular parou de dar sinal (o ping dele chega todo segundo)
                _ = sleep_until(limite) => break,
                msg = ws.recv() => {
                    let Some(Ok(msg)) = msg else { break };
                    limite = Instant::now() + inatividade;
                    // aparelho revogado enquanto conectado: corta na hora
                    let Some(permissoes) = acesso.permissoes(&token) else {
                        recusar(&mut ws).await;
                        break;
                    };
                    let Message::Binary(dados) = msg else { continue };
                    match decodificar(&dados) {
                        Some(Evento::Ping) => {
                            if ws.send(Message::Binary(dados)).await.is_err() {
                                break;
                            }
                        }
                        Some(Evento::TrocarPerfil(indice)) if e_perfil_de_teclado(indice) => {
                            if pedir_teclado(&mut ws, &estado, &token, indice, origem, &retorno_tx, &mut ativacao)
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        // com o teclado ativo, só quem tem permissão digita; os outros seguem
                        // conectados e podem, por exemplo, trocar de perfil
                        Some(evento @ Evento::TrocarPerfil(_)) => {
                            let _ = saida.send(evento);
                        }
                        Some(evento) => {
                            if permissoes.teclado || !modo_teclado_ativo(&perfil) {
                                let _ = saida.send(evento);
                            }
                        }
                        None => {}
                    }
                }
                Some(aviso) = retorno_rx.recv() => {
                    if enviar(&mut ws, aviso).await.is_err() {
                        break;
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
    // O celular foi embora: um pedido de teclado que ainda esperava o "sim" deixa de existir.
    if let Some(tarefa) = ativacao {
        tarefa.abort();
    }
    // Soltar tudo evita botões "presos" se o celular some no meio de um toque.
    let _ = saida.send(Evento::SoltarTudo);
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
        .route("/api/parear", post(api::parear))
        .route("/api/teclado", get(api::obter).put(api::definir))
        .with_state(Estado { saida, perfil, mapa, acesso })
        .fallback(get(estatico))
}

/// Servidor que descarta os eventos (só serve a página). Ninguém está pareado, então nenhuma
/// conexão de controle nem chamada à API é aceita.
pub fn app() -> Router {
    app_com(
        mpsc::unbounded_channel().0,
        watch::channel(0).1,
        Arc::new(ArmazemMapa::em_memoria()),
        Acesso::novo(
            Dispositivos::em_memoria(),
            Pareamento::novo().expect("sem /dev/urandom não há como gerar o código de pareamento"),
            Duration::from_secs(6),
        ),
    )
}
