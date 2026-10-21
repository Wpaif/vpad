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
