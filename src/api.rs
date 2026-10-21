//! API HTTP do modo teclado: `GET /api/teclado` e `PUT /api/teclado`.

use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::{
    net::SocketAddr,
    time::{SystemTime, UNIX_EPOCH},
};
use serde_json::json;
use std::collections::BTreeMap;

use crate::{
    Estado,
    acesso::Pareamento_,
    mapa::{Controle, Mapa},
    teclas::TECLAS,
};

/// O token vem em `Authorization: Bearer <token>`.
fn token_do_cabecalho(headers: &HeaderMap) -> Option<&str> {
    headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "))
}

/// Confere o token e exige a permissão do modo teclado: ler e mudar as teclas é coisa de quem
/// pode digitar no PC. Devolve a resposta de erro pronta se não passar.
pub(crate) fn exigir_teclado(estado: &Estado, headers: &HeaderMap) -> Result<(), Response> {
    let permissoes = token_do_cabecalho(headers).and_then(|t| estado.acesso.permissoes(t, agora()));
    match permissoes {
        None => Err(erro(StatusCode::UNAUTHORIZED, "não autorizado")),
        Some(p) if !p.teclado => Err(erro(
            StatusCode::FORBIDDEN,
            "este aparelho não tem permissão para o modo teclado: libere no PC com vpad --permitir-teclado <id>",
        )),
        Some(_) => Ok(()),
    }
}

/// Mapa atual, o padrão, a lista de controles (na ordem da página) e o catálogo de teclas.
pub(crate) async fn obter(State(estado): State<Estado>, headers: HeaderMap) -> Response {
    if let Err(resposta) = exigir_teclado(&estado, &headers) {
        return resposta;
    }
    let teclas: Vec<_> = TECLAS
        .iter()
        .map(|t| json!({ "id": t.id, "rotulo": t.rotulo, "grupo": t.grupo }))
        .collect();
    let controles: Vec<_> = Controle::todos().iter().map(|c| c.nome()).collect();
    Json(json!({
        "mapa": estado.mapa.atual().para_json(),
        "padrao": Mapa::padrao().para_json(),
        "controles": controles,
        "teclas": teclas,
    }))
    .into_response()
}

#[derive(Deserialize)]
pub(crate) struct Pedido {
    mapa: BTreeMap<String, String>,
}

fn erro(status: StatusCode, mensagem: impl ToString) -> Response {
    (status, Json(json!({ "erro": mensagem.to_string() }))).into_response()
}

pub(crate) async fn definir(
    State(estado): State<Estado>,
    headers: HeaderMap,
    Json(pedido): Json<Pedido>,
) -> Response {
    if let Err(resposta) = exigir_teclado(&estado, &headers) {
        return resposta;
    }
    let mapa = match Mapa::de_json(&pedido.mapa) {
        Ok(m) => m,
        Err(e) => return erro(StatusCode::BAD_REQUEST, e),
    };
    let salvo = mapa.para_json();
    match estado.mapa.definir(mapa) {
        Ok(()) => Json(json!({ "mapa": salvo })).into_response(),
        Err(e) => erro(StatusCode::INTERNAL_SERVER_ERROR, format!("não consegui gravar: {e}")),
    }
}

#[derive(Deserialize)]
pub(crate) struct PedidoPareamento {
    codigo: String,
    nome: Option<String>,
}

fn agora() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Troca o código de 6 dígitos do terminal por um token exclusivo do aparelho. Não exige
/// token (é assim que o aparelho consegue o primeiro), mas erra com limite de tentativas.
pub(crate) async fn parear(
    State(estado): State<Estado>,
    partes: Parts,
    Json(pedido): Json<PedidoPareamento>,
) -> Response {
    let nome = pedido.nome.unwrap_or_default();
    // o servidor HTTPS põe o endereço de quem conectou nas extensões da requisição
    let origem = partes.extensions.get::<ConnectInfo<SocketAddr>>().map(|c| c.0.ip());
    // com o código certo, a resposta só sai depois que quem está no PC aprova (ou não)
    match estado.acesso.parear(&pedido.codigo, &nome, agora(), origem).await {
        Pareamento_::Pareado(p) => Json(json!({ "token": p.token, "id": p.id })).into_response(),
        Pareamento_::Incorreto => erro(StatusCode::UNAUTHORIZED, "código incorreto ou expirado"),
        Pareamento_::Fechado => erro(
            StatusCode::FORBIDDEN,
            "o pareamento está fechado: no PC, pressione Enter no terminal do vpad para abri-lo",
        ),
        Pareamento_::Negado => erro(StatusCode::FORBIDDEN, "o pedido foi negado no PC"),
        Pareamento_::Expirou => erro(
            StatusCode::REQUEST_TIMEOUT,
            "ninguém aprovou o pedido no PC a tempo: abra o pareamento e tente de novo",
        ),
        Pareamento_::Ocupado => erro(
            StatusCode::CONFLICT,
            "já há outro pedido de pareamento aguardando aprovação no PC",
        ),
        Pareamento_::Bloqueado { segundos } => (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, segundos.to_string())],
            Json(json!({ "erro": "muitas tentativas erradas", "segundos": segundos })),
        )
            .into_response(),
        Pareamento_::Falha(e) => erro(StatusCode::INTERNAL_SERVER_ERROR, format!("não consegui gravar: {e}")),
    }
}
