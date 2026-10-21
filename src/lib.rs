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
    body::Body,
    extract::{
        ConnectInfo, DefaultBodyLimit, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderValue, Request, StatusCode, Uri, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rust_embed::RustEmbed;
use std::{net::{IpAddr, SocketAddr}, sync::Arc, time::Duration};
use tokio::{
    sync::{
        mpsc::{self, Sender},
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

/// Uma mensagem só carrega um botão (3), um eixo (4), um ping (9) ou um perfil (2). O teto
/// fica bem acima disso, mas longe de permitir que um cliente gere tráfego arbitrário.
const TAMANHO_MAXIMO_DA_MENSAGEM: usize = 64;
/// O maior corpo JSON aceito (o mapa de teclas inteiro tem poucos KB).
const TAMANHO_MAXIMO_DO_CORPO: usize = 16 * 1024;

type Saida = Sender<Evento>;

/// O `Host` que o celular usou é um endereço desta rede local? Serve para barrar *DNS
/// rebinding*: uma página da internet que resolve para o IP do PC manda outro nome e é recusada.
fn host_permitido(host: &str) -> bool {
    let host = host.trim_end_matches('.');
    if let Ok(ip) = host.parse::<IpAddr>() {
        return ip.is_loopback() || crate::tls::ip_permitido(ip);
    }
    let nome = host.to_ascii_lowercase();
    nome == "localhost" || nome.ends_with(".local") || nome == crate::tls::nome_do_computador().to_ascii_lowercase()
}

/// O nome de um cabeçalho `Host`/`Origin`, sem porta e sem colchetes de IPv6.
fn nome_do_host(valor: &str) -> Option<String> {
    let valor = valor.trim();
    if let Some(resto) = valor.strip_prefix('[') {
        return resto.split(']').next().map(str::to_string).filter(|h| !h.is_empty());
    }
    let nome = valor.rsplit_once(':').map_or(valor, |(nome, _porta)| nome);
    (!nome.is_empty()).then(|| nome.to_string())
}

/// A `Origin` é de uma página servida por este mesmo PC? Sem `Origin` (clientes que não são
/// navegador) não há o que conferir. `Origin: null` e páginas da internet são recusadas.
fn origem_permitida(origem: &str) -> bool {
    if origem == "null" {
        return false;
    }
    let Ok(uri) = origem.parse::<Uri>() else { return false };
    if uri.scheme_str() != Some("https") {
        return false;
    }
    uri.host().is_some_and(host_permitido)
}

/// Porteiro de todas as rotas: só aceita pedidos endereçados a esta rede local e vindos de
/// uma página dela. É o que impede um site qualquer de falar com o vpad pela rede.
async fn porteiro(State(_estado): State<Estado>, pedido: Request<Body>, seguinte: Next) -> Response {
    if let Some(host) = pedido.headers().get(header::HOST) {
        match host.to_str().ok().and_then(nome_do_host) {
            Some(nome) if host_permitido(&nome) => {}
            _ => return (StatusCode::MISDIRECTED_REQUEST, "Host não permitido").into_response(),
        }
    }
    if let Some(origem) = pedido.headers().get(header::ORIGIN) {
        match origem.to_str() {
            Ok(origem) if origem_permitida(origem) => {}
            _ => return (StatusCode::FORBIDDEN, "Origin não permitido").into_response(),
        }
    }
    seguinte.run(pedido).await
}

/// Autentica antes de o corpo da requisição ser lido: um token inválido não chega a fazer o
/// servidor interpretar (nem guardar) o JSON enviado.
async fn exigir_token_do_teclado(State(estado): State<Estado>, pedido: Request<Body>, seguinte: Next) -> Response {
    match crate::api::exigir_teclado(&estado, pedido.headers()) {
        Ok(()) => seguinte.run(pedido).await,
        Err(resposta) => resposta,
    }
}

/// Cabeçalhos de segurança em toda resposta do HTTPS.
async fn cabecalhos_de_seguranca(mut resposta: Response) -> Response {
    let cabecalhos = resposta.headers_mut();
    cabecalhos.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; \
             connect-src 'self' wss:; manifest-src 'self'; base-uri 'none'; form-action 'self'; \
             frame-ancestors 'none'",
        ),
    );
    cabecalhos.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    cabecalhos.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    cabecalhos.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    cabecalhos.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=(), usb=()"),
    );
    cabecalhos.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000"),
    );
    resposta
}

#[derive(Clone)]
struct Estado {
    saida: Saida,
    /// Índice do perfil ativo, publicado pelo serviço dono do controle virtual.
    perfil: watch::Receiver<u8>,
    mapa: Arc<ArmazemMapa>,
    acesso: Arc<Acesso>,
}

/// O token do aparelho viaja no subprotocolo `vpad.<token>`, nunca na URL — que acaba no
/// histórico do navegador, no Referer e nos logs. O `vpad` puro é o nome que o servidor
/// escolhe na negociação; o token fica no outro valor oferecido pelo cliente.
fn token_do_subprotocolo(partes: &axum::http::request::Parts) -> Option<String> {
    partes
        .headers
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter_map(|valor| valor.to_str().ok())
        .flat_map(|valor| valor.split(','))
        .find_map(|protocolo| protocolo.trim().strip_prefix("vpad."))
        .filter(|token| token.len() == 32 && token.chars().all(|c| c.is_ascii_hexdigit()))
        .map(str::to_string)
}

fn ofereceu_vpad(partes: &axum::http::request::Parts) -> bool {
    partes
        .headers
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter_map(|valor| valor.to_str().ok())
        .flat_map(|valor| valor.split(','))
        .any(|protocolo| protocolo.trim() == "vpad")
}

async fn rota_ws(
    ws: WebSocketUpgrade,
    State(estado): State<Estado>,
    partes: axum::http::request::Parts,
) -> Response {
    let origem = partes.extensions.get::<ConnectInfo<SocketAddr>>().map(|c| c.0.ip());
    let token = token_do_subprotocolo(&partes).unwrap_or_default();
    // só escolhe `vpad` se o cliente o ofereceu; senão o handshake seria recusado pelo
    // navegador (que exige um subprotocolo dentre os pedidos) antes de podermos recusar
    let ws = if ofereceu_vpad(&partes) { ws.protocols(["vpad"]) } else { ws };
    ws.max_message_size(TAMANHO_MAXIMO_DA_MENSAGEM)
        .max_frame_size(TAMANHO_MAXIMO_DA_MENSAGEM)
        .on_upgrade(move |socket| conexao(socket, estado, token, origem))
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
    retorno: &mpsc::Sender<Vec<u8>>,
    ativacao: &mut Option<tokio::task::JoinHandle<()>>,
) -> Result<(), axum::Error> {
    let acesso = &estado.acesso;
    let Some(aparelho) = acesso.dispositivos().identificar(token, crate::dispositivos::agora()) else { return Ok(()) };
    if !aparelho.teclado {
        return enviar(ws, codificar_aviso_de_perfil(AvisoDePerfil::SemPermissao)).await;
    }
    if *estado.perfil.borrow() == indice {
        let _ = estado.saida.send(Evento::TrocarPerfil(indice)).await; // já está no modo teclado
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
                let _ = saida.send(Evento::TrocarPerfil(indice)).await;
                return;
            }
            Ok(Decisao::Negado) => AvisoDePerfil::Negado,
            Ok(Decisao::Expirou) => AvisoDePerfil::Expirou,
            Err(Ocupado) => AvisoDePerfil::Ocupado,
        };
        let _ = retorno.send(codificar_aviso_de_perfil(aviso).to_vec()).await;
    }));
    Ok(())
}

async fn conexao(mut ws: WebSocket, estado: Estado, token: String, origem: Option<IpAddr>) {
    if !estado.acesso.autorizado(&token, crate::dispositivos::agora()) {
        return recusar(&mut ws).await;
    }
    let saida = estado.saida.clone();
    let acesso = estado.acesso.clone();
    let mut perfil = estado.perfil.clone();

    // respostas do pedido de ativação do teclado, que espera o "sim" no PC em outra tarefa
    let (retorno_tx, mut retorno_rx) = mpsc::channel::<Vec<u8>>(8);
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
                    let Some(permissoes) = acesso.permissoes(&token, crate::dispositivos::agora()) else {
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
                            let _ = saida.send(evento).await;
                        }
                        Some(evento) => {
                            if permissoes.teclado || !modo_teclado_ativo(&perfil) {
                                let _ = saida.send(evento).await;
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
    let _ = saida.send(Evento::SoltarTudo).await;
}

/// Servidor cujos eventos de controle vão para `saida`; `perfil` informa o perfil ativo,
/// `mapa` guarda as teclas do modo teclado e `acesso` decide quem pode controlar.
pub fn app_com(
    saida: Saida,
    perfil: watch::Receiver<u8>,
    mapa: Arc<ArmazemMapa>,
    acesso: Arc<Acesso>,
) -> Router {
    let estado = Estado { saida, perfil, mapa, acesso };
    let teclado = get(api::obter)
        .put(api::definir)
        .route_layer(middleware::from_fn_with_state(estado.clone(), exigir_token_do_teclado));
    Router::new()
        .route("/ws", get(rota_ws))
        .route("/api/parear", post(api::parear))
        .route("/api/teclado", teclado)
        .fallback(get(estatico))
        .with_state(estado.clone())
        .layer(middleware::from_fn_with_state(estado, porteiro))
        .layer(middleware::map_response(cabecalhos_de_seguranca))
        .layer(DefaultBodyLimit::max(TAMANHO_MAXIMO_DO_CORPO))
}

/// Vigia o arquivo do cadastro de aparelhos para que `vpad --revogar` (outro processo) valha
/// no mesmo instante, sem esperar a releitura periódica. Devolve a tarefa que observa o
/// arquivo; `None` quando o cadastro não está em disco (memória).
pub fn vigiar_revogacoes(acesso: Arc<Acesso>) -> std::io::Result<Option<tokio::task::JoinHandle<()>>> {
    use futures_util::StreamExt;
    use inotify::{Inotify, WatchMask};

    let Some(arquivo) = acesso.dispositivos().caminho().map(|c| c.to_path_buf()) else { return Ok(None) };
    let Some(pasta) = arquivo.parent().map(|p| p.to_path_buf()) else { return Ok(None) };

    let inotify = Inotify::init()?;
    inotify.watches().add(&pasta, WatchMask::CLOSE_WRITE | WatchMask::MOVED_TO | WatchMask::CREATE)?;
    let mut eventos = inotify.into_event_stream([0u8; 1024])?;
    Ok(Some(tokio::spawn(async move {
        while let Some(Ok(evento)) = eventos.next().await {
            let do_arquivo = evento.name.as_deref().is_none_or(|nome| arquivo.file_name() == Some(nome));
            if do_arquivo {
                acesso.dispositivos().recarregar();
            }
        }
    })))
}

/// Servidor que descarta os eventos (só serve a página). Ninguém está pareado, então nenhuma
/// conexão de controle nem chamada à API é aceita.
pub fn app() -> Router {
    app_com(
        mpsc::channel(1).0,
        watch::channel(0).1,
        Arc::new(ArmazemMapa::em_memoria()),
        Acesso::novo(
            Dispositivos::em_memoria(),
            Pareamento::novo().expect("sem /dev/urandom não há como gerar o código de pareamento"),
            Duration::from_secs(6),
        ),
    )
}
