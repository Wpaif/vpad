//! API HTTP do modo teclado: `GET /api/teclado` e `PUT /api/teclado`.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::json;
use std::collections::BTreeMap;

use crate::{
    Estado,
    acesso::Pareamento_,
    mapa::{Controle, Mapa},
    teclas::TECLAS,
};

/// O token vem em `Authorization: Bearer <token>`.
fn autorizado(estado: &Estado, headers: &HeaderMap) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|token| estado.acesso.autorizado(token))
}

fn nao_autorizado() -> Response {
    erro(StatusCode::UNAUTHORIZED, "não autorizado")
}

/// Mapa atual, o padrão, a lista de controles (na ordem da página) e o catálogo de teclas.
pub(crate) async fn obter(State(estado): State<Estado>, headers: HeaderMap) -> Response {
    if !autorizado(&estado, &headers) {
        return nao_autorizado();
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
    if !autorizado(&estado, &headers) {
        return nao_autorizado();
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
pub(crate) async fn parear(State(estado): State<Estado>, Json(pedido): Json<PedidoPareamento>) -> Response {
    let nome = pedido.nome.unwrap_or_default();
    match estado.acesso.parear(&pedido.codigo, &nome, agora()) {
        Pareamento_::Pareado(p) => {
            println!("\ndispositivo pareado: {} (id {})", estado.acesso.dispositivos().nome_de(&p.id), p.id);
            Json(json!({ "token": p.token, "id": p.id })).into_response()
        }
        Pareamento_::Incorreto => erro(StatusCode::UNAUTHORIZED, "código incorreto ou expirado"),
        Pareamento_::Bloqueado { segundos } => (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, segundos.to_string())],
            Json(json!({ "erro": "muitas tentativas erradas", "segundos": segundos })),
        )
            .into_response(),
        Pareamento_::Falha(e) => erro(StatusCode::INTERNAL_SERVER_ERROR, format!("não consegui gravar: {e}")),
    }
}
