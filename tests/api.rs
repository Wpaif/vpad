use std::{fs, path::PathBuf, sync::Arc};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tokio::sync::{mpsc, watch};
use tower::ServiceExt;
use vpad::{acesso::Acesso, armazem::ArmazemMapa, mapa::Mapa};

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

fn app(armazem: Arc<ArmazemMapa>) -> Router {
    vpad::app_com(
        mpsc::unbounded_channel().0,
        watch::channel(0).1,
        armazem,
        Acesso::novo(TOKEN, std::time::Duration::from_secs(6)),
    )
}

async fn chamar(app: &Router, metodo: &str, corpo: Option<&str>) -> (StatusCode, Value) {
    chamar_com(app, metodo, corpo, Some(&format!("Bearer {TOKEN}"))).await
}

async fn chamar_com(
    app: &Router,
    metodo: &str,
    corpo: Option<&str>,
    autorizacao: Option<&str>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(metodo).uri("/api/teclado");
    if let Some(a) = autorizacao {
        req = req.header(header::AUTHORIZATION, a);
    }
    let body = match corpo {
        Some(c) => {
            req = req.header(header::CONTENT_TYPE, "application/json");
            Body::from(c.to_string())
        }
        None => Body::empty(),
    };
    let res = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn pedido(mapa: Value) -> String {
    json!({ "mapa": mapa }).to_string()
}

#[tokio::test]
async fn get_devolve_mapa_padrao_controles_e_catalogo_de_teclas() {
    let app = app(Arc::new(ArmazemMapa::em_memoria()));
    let (status, corpo) = chamar(&app, "GET", None).await;

    assert_eq!(status, StatusCode::OK);
    let padrao = serde_json::to_value(Mapa::padrao().para_json()).unwrap();
    assert_eq!(corpo["mapa"], padrao);
    assert_eq!(corpo["padrao"], padrao);

    let controles: Vec<_> = corpo["controles"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    assert_eq!(controles.len(), 24);
    assert_eq!(controles[0], "A");
    assert_eq!(controles[23], "RS_Right");

    let teclas = corpo["teclas"].as_array().unwrap();
    assert_eq!(teclas.len(), vpad::teclas::TECLAS.len());
    assert!(teclas.contains(&json!({"id": "KeyA", "rotulo": "A", "grupo": "Letras"})));
}

#[tokio::test]
async fn put_valido_salva_responde_e_aparece_no_get() {
    let armazem = Arc::new(ArmazemMapa::em_memoria());
    let assinante = armazem.assinar();
    let app = app(armazem.clone());

    let (status, corpo) = chamar(&app, "PUT", Some(&pedido(json!({"A": "Space", "RS_Up": "KeyO"})))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(corpo["mapa"], json!({"A": "Space", "RS_Up": "KeyO"}));

    assert!(assinante.has_changed().unwrap(), "o serviço precisa ser avisado");
    let (_, depois) = chamar(&app, "GET", None).await;
    assert_eq!(depois["mapa"], json!({"A": "Space", "RS_Up": "KeyO"}));
    assert_ne!(depois["padrao"], depois["mapa"], "o padrão não muda");
}

#[tokio::test]
async fn put_com_mapa_vazio_desmapeia_tudo() {
    let app = app(Arc::new(ArmazemMapa::em_memoria()));
    let (status, corpo) = chamar(&app, "PUT", Some(&pedido(json!({})))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(corpo["mapa"], json!({}));
}

#[tokio::test]
async fn put_com_tecla_ou_controle_desconhecido_e_400_e_nao_muda_nada() {
    let armazem = Arc::new(ArmazemMapa::em_memoria());
    let app = app(armazem.clone());

    let (status, corpo) = chamar(&app, "PUT", Some(&pedido(json!({"A": "Hyper"})))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(corpo["erro"].as_str().unwrap().contains("Hyper"));

    let (status, corpo) = chamar(&app, "PUT", Some(&pedido(json!({"Turbo": "KeyA"})))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(corpo["erro"].as_str().unwrap().contains("Turbo"));

    assert_eq!(*armazem.atual(), Mapa::padrao());
}

#[tokio::test]
async fn put_com_corpo_malformado_e_rejeitado() {
    let app = app(Arc::new(ArmazemMapa::em_memoria()));
    for corpo in ["isto nao e json", r#"{"mapa": 3}"#, r#"{"outro": {}}"#] {
        let (status, _) = chamar(&app, "PUT", Some(corpo)).await;
        assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY, "{corpo}: {status}");
    }
}

#[tokio::test]
async fn falha_ao_gravar_em_disco_e_500_e_o_mapa_continua() {
    let pasta: PathBuf = std::env::temp_dir().join(format!("vpad-api-{}", std::process::id()));
    let _ = fs::remove_dir_all(&pasta);
    fs::create_dir_all(&pasta).unwrap();
    fs::write(pasta.join("vpad"), "x").unwrap(); // "vpad" é arquivo: não dá para criar nada dentro
    let armazem = Arc::new(ArmazemMapa::com_arquivo(pasta.join("vpad").join("teclado.json")));
    let app = app(armazem.clone());

    let (status, corpo) = chamar(&app, "PUT", Some(&pedido(json!({"A": "Space"})))).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(corpo["erro"].is_string());
    assert_eq!(*armazem.atual(), Mapa::padrao());
    let _ = fs::remove_dir_all(&pasta);
}

#[tokio::test]
async fn outros_metodos_nao_sao_aceitos() {
    let app = app(Arc::new(ArmazemMapa::em_memoria()));
    let (status, _) = chamar(&app, "DELETE", None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn sem_o_token_certo_a_api_responde_401_e_nao_muda_nada() {
    let armazem = Arc::new(ArmazemMapa::em_memoria());
    let app = app(armazem.clone());

    for autorizacao in [
        None,
        Some("Bearer errado"),
        Some("Bearer "),
        Some(TOKEN),                          // sem o prefixo Bearer
        Some("Basic 0123456789abcdef0123456789abcdef"),
    ] {
        let (status, corpo) = chamar_com(&app, "GET", None, autorizacao).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "GET com {autorizacao:?}");
        assert!(corpo["erro"].is_string());

        let pedido = pedido(json!({"A": "Space"}));
        let (status, _) = chamar_com(&app, "PUT", Some(&pedido), autorizacao).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "PUT com {autorizacao:?}");
    }
    assert_eq!(*armazem.atual(), Mapa::padrao());
}
