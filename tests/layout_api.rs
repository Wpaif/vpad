//! API do layout dos botões: `GET /api/layout` e `PUT /api/layout`, abertas a qualquer
//! aparelho pareado (não dependem da permissão do modo teclado).

use std::{
    fs,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tokio::sync::{mpsc, watch};
use tower::ServiceExt;
use vpad::{
    acesso::Acesso, armazem::ArmazemMapa, dispositivos::Dispositivos, layout::ArmazemLayout,
    pareamento::Pareamento,
};

struct Ambiente {
    app: Router,
    acesso: Arc<Acesso>,
    /// token de um aparelho pareado SEM permissão de teclado
    token: String,
    id: String,
}

fn agora() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn ambiente_com(layout: Arc<ArmazemLayout>) -> Ambiente {
    let acesso = Acesso::novo(
        Dispositivos::em_memoria(),
        Pareamento::com_segredo(b"segredo-de-teste-123".to_vec()),
        Duration::from_secs(6),
    );
    let pareado = acesso.dispositivos().parear_com("teste", agora(), false).unwrap();
    let app = vpad::app_completo(
        mpsc::channel(64).0,
        watch::channel(0).1,
        Arc::new(ArmazemMapa::em_memoria()),
        layout,
        acesso.clone(),
    );
    Ambiente { app, acesso, token: pareado.token, id: pareado.id }
}

fn ambiente() -> Ambiente {
    ambiente_com(Arc::new(ArmazemLayout::em_memoria()))
}

async fn chamar(app: &Router, metodo: &str, corpo: Option<String>, token: Option<&str>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(metodo).uri("/api/layout");
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let body = match corpo {
        Some(c) => {
            req = req.header(header::CONTENT_TYPE, "application/json");
            Body::from(c)
        }
        None => Body::empty(),
    };
    let res = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn layout_completo() -> Value {
    json!({
        "version": 1,
        "groups": { "face": { "dx": -0.1, "dy": 0.05, "scale": 1.3 } },
        "hidden": ["A"],
        "controls": { "B": { "dx": 0.04, "dy": -0.1 } }
    })
}

fn pedido(layout: Value) -> String {
    json!({ "layout": layout }).to_string()
}

#[tokio::test]
async fn sem_token_e_401_no_get_e_no_put() {
    let a = ambiente();
    let (status, corpo) = chamar(&a.app, "GET", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(corpo["codigo"], "nao_autorizado");
    assert_eq!(chamar(&a.app, "PUT", Some(pedido(layout_completo())), None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(chamar(&a.app, "GET", None, Some("0123456789abcdef0123456789abcdef")).await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sem_layout_salvo_o_get_devolve_null() {
    let a = ambiente();
    let (status, corpo) = chamar(&a.app, "GET", None, Some(&a.token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(corpo, json!({ "layout": null }));
}

#[tokio::test]
async fn put_valido_salva_e_aparece_no_get() {
    let a = ambiente();
    let (status, corpo) = chamar(&a.app, "PUT", Some(pedido(layout_completo())), Some(&a.token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(corpo, json!({ "layout": layout_completo() }));
    let (_, depois) = chamar(&a.app, "GET", None, Some(&a.token)).await;
    assert_eq!(depois["layout"], layout_completo());
}

#[tokio::test]
async fn aparelho_sem_permissao_de_teclado_pode_usar_o_layout() {
    // o token do ambiente foi pareado sem teclado: o layout não depende dessa permissão
    let a = ambiente();
    assert!(!a.acesso.permissoes(&a.token, agora()).unwrap().teclado);
    assert_eq!(chamar(&a.app, "PUT", Some(pedido(layout_completo())), Some(&a.token)).await.0, StatusCode::OK);
}

#[tokio::test]
async fn layout_invalido_e_400_e_nao_muda_o_que_estava_salvo() {
    let a = ambiente();
    chamar(&a.app, "PUT", Some(pedido(layout_completo())), Some(&a.token)).await;
    for ruim in [
        json!([1, 2]),
        json!({ "version": 0 }),
        json!({ "version": 1, "extra": true }),
        json!({ "version": 1, "groups": { "../x": { "dx": 0 } } }),
        json!({ "version": 1, "groups": { "face": { "dx": 9 } } }),
    ] {
        let (status, corpo) = chamar(&a.app, "PUT", Some(pedido(ruim.clone())), Some(&a.token)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{ruim}");
        assert!(corpo["erro"].is_string());
        assert_eq!(corpo["codigo"], "layout_invalido");
    }
    let (_, depois) = chamar(&a.app, "GET", None, Some(&a.token)).await;
    assert_eq!(depois["layout"], layout_completo());
}

#[tokio::test]
async fn corpo_sem_o_campo_layout_ou_que_nao_e_json_e_recusado() {
    let a = ambiente();
    for corpo in [json!({}).to_string(), json!({ "outro": 1 }).to_string(), "isto nao e json".to_string()] {
        let (status, _) = chamar(&a.app, "PUT", Some(corpo.clone()), Some(&a.token)).await;
        assert!(status.is_client_error(), "{corpo}: {status}");
    }
}

#[tokio::test]
async fn corpo_grande_demais_e_recusado() {
    let a = ambiente();
    let enorme = json!({ "layout": { "version": 1, "hidden": ["A"], "lixo": "x".repeat(40_000) } }).to_string();
    let (status, _) = chamar(&a.app, "PUT", Some(enorme), Some(&a.token)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn aparelho_revogado_perde_o_acesso() {
    let a = ambiente();
    assert_eq!(chamar(&a.app, "GET", None, Some(&a.token)).await.0, StatusCode::OK);
    assert!(a.acesso.dispositivos().revogar(&a.id).unwrap());
    assert_eq!(chamar(&a.app, "GET", None, Some(&a.token)).await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn o_layout_e_um_so_para_todos_os_aparelhos_e_sobrevive_ao_reinicio() {
    let pasta = std::env::temp_dir().join(format!("vpad-layout-api-{}", std::process::id()));
    let _ = fs::remove_dir_all(&pasta);
    let arquivo = pasta.join("vpad").join("layout.json");

    let a = ambiente_com(Arc::new(ArmazemLayout::com_arquivo(arquivo.clone())));
    chamar(&a.app, "PUT", Some(pedido(layout_completo())), Some(&a.token)).await;
    assert!(arquivo.exists());

    // outro aparelho, outra "execução" do servidor: lê o mesmo layout do arquivo
    let b = ambiente_com(Arc::new(ArmazemLayout::com_arquivo(arquivo.clone())));
    let (status, corpo) = chamar(&b.app, "GET", None, Some(&b.token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(corpo["layout"], layout_completo());
    let _ = fs::remove_dir_all(&pasta);
}

#[tokio::test]
async fn falha_de_disco_no_put_e_500_e_nao_muda_o_layout() {
    let pasta = std::env::temp_dir().join(format!("vpad-layout-api-falha-{}", std::process::id()));
    let _ = fs::remove_dir_all(&pasta);
    fs::create_dir_all(&pasta).unwrap();
    // "vpad" é um arquivo: não dá para criar layout.json dentro dele
    fs::write(pasta.join("vpad"), "x").unwrap();
    let a = ambiente_com(Arc::new(ArmazemLayout::com_arquivo(pasta.join("vpad").join("layout.json"))));

    let (status, corpo) = chamar(&a.app, "PUT", Some(pedido(layout_completo())), Some(&a.token)).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(corpo["erro"].is_string());
    assert_eq!(corpo["codigo"], "gravacao_falhou");
    assert_eq!(chamar(&a.app, "GET", None, Some(&a.token)).await.1, json!({ "layout": null }));
    let _ = fs::remove_dir_all(&pasta);
}

#[tokio::test]
async fn opacidade_dos_botoes_e_guardada_e_devolvida() {
    let a = ambiente();
    let layout = json!({ "version": 1, "groups": {}, "hidden": [], "opacity": 0.6 });
    let (status, corpo) = chamar(&a.app, "PUT", Some(pedido(layout.clone())), Some(&a.token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(corpo["layout"]["opacity"], json!(0.6));
    assert_eq!(chamar(&a.app, "GET", None, Some(&a.token)).await.1["layout"], layout);

    for ruim in [json!(5), json!(0.1), json!("0.5")] {
        let l = json!({ "version": 1, "opacity": ruim.clone() });
        assert_eq!(chamar(&a.app, "PUT", Some(pedido(l)), Some(&a.token)).await.0, StatusCode::BAD_REQUEST, "{ruim}");
    }
}
