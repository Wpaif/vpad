//! API HTTP do modo teclado: `GET /api/teclado` e `PUT /api/teclado`.

use axum::{Json, extract::State, http::StatusCode, response::{IntoResponse, Response}};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

use crate::{
    Estado,
    mapa::{Controle, Mapa},
    teclas::TECLAS,
};

/// Mapa atual, o padrão, a lista de controles (na ordem da página) e o catálogo de teclas.
pub(crate) async fn obter(State(estado): State<Estado>) -> Json<Value> {
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
}

#[derive(Deserialize)]
pub(crate) struct Pedido {
    mapa: BTreeMap<String, String>,
}

fn erro(status: StatusCode, mensagem: impl ToString) -> Response {
    (status, Json(json!({ "erro": mensagem.to_string() }))).into_response()
}

pub(crate) async fn definir(State(estado): State<Estado>, Json(pedido): Json<Pedido>) -> Response {
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
