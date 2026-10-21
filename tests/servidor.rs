use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn get(path: &str) -> axum::response::Response {
    vpad::app()
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn raiz_serve_a_pagina_html() {
    let res = get("/").await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()[header::CONTENT_TYPE],
        "text/html; charset=utf-8"
    );
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("<title>vpad"));
}

#[tokio::test]
async fn arquivo_inexistente_retorna_404() {
    assert_eq!(get("/nao-existe.js").await.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn serve_assets_com_o_content_type_correto() {
    let res = get("/index.html").await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers()[header::CONTENT_TYPE]
        .to_str()
        .unwrap()
        .starts_with("text/html"));
}

#[tokio::test]
async fn serve_o_manifest_do_pwa_em_tela_cheia() {
    let res = get("/manifest.webmanifest").await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers()[header::CONTENT_TYPE]
        .to_str()
        .unwrap()
        .starts_with("application/manifest+json"));
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let texto = String::from_utf8_lossy(&body);
    assert!(texto.contains("\"display\": \"fullscreen\""));
    assert!(texto.contains("\"orientation\": \"landscape\""));
}

#[tokio::test]
async fn serve_o_icone_para_a_tela_inicial_do_iphone() {
    let res = get("/apple-touch-icon.png").await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "image/png");
}

#[tokio::test]
async fn toda_resposta_traz_cabecalhos_de_seguranca() {
    for path in ["/", "/index.html", "/manifest.webmanifest", "/nao-existe.js"] {
        let res = get(path).await;
        let headers = res.headers();
        let csp = headers[header::CONTENT_SECURITY_POLICY].to_str().unwrap();
        assert!(csp.contains("default-src 'none'"), "{path}: {csp}");
        assert!(csp.contains("script-src 'self'"), "{path}: {csp}");
        assert!(csp.contains("frame-ancestors 'none'"), "{path}: {csp}");
        assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff", "{path}");
        assert_eq!(headers[header::X_FRAME_OPTIONS], "DENY", "{path}");
        assert_eq!(headers[header::REFERRER_POLICY], "no-referrer", "{path}");
        assert_eq!(headers[header::STRICT_TRANSPORT_SECURITY], "max-age=31536000", "{path}");
    }
}
