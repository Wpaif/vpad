//! Página de configuração, em HTTP simples: só entrega a autoridade do vpad (para o celular
//! passar a confiar no HTTPS) e as instruções. Nada do controle é servido por aqui: o
//! WebSocket, o pareamento e as teclas existem apenas no HTTPS.

use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};

use crate::mobileconfig::perfil_ios;

pub struct Configuracao {
    /// certificado da autoridade, em DER e em PEM
    pub ca_der: Vec<u8>,
    pub ca_pem: String,
    pub nome_do_pc: String,
    pub porta_https: u16,
}

/// O nome do servidor que o celular usou para chegar aqui (sem a porta), se for um endereço
/// simples. Qualquer coisa fora de letras, dígitos, ponto e hífen é descartada: o valor vai
/// parar dentro de um link na página.
fn endereco_seguro(host: &str) -> Option<&str> {
    let nome = host.rsplit_once(':').map_or(host, |(nome, _porta)| nome);
    let simples = !nome.is_empty()
        && nome.len() <= 253
        && nome.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    simples.then_some(nome)
}

async fn pagina(State(cfg): State<Arc<Configuracao>>, headers: HeaderMap) -> Html<String> {
    let host = headers.get(header::HOST).and_then(|h| h.to_str().ok()).and_then(endereco_seguro);
    let abrir = match host {
        Some(h) => format!(r#"<a class="botao" href="https://{h}:{}/">Abrir o controle</a>"#, cfg.porta_https),
        None => format!("Abra o endereço do PC com <b>https</b> e a porta <b>{}</b>.", cfg.porta_https),
    };
    Html(format!(
        r#"<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>vpad · configurar HTTPS</title>
<style>
  :root {{ color-scheme: dark; font-family: -apple-system, system-ui, sans-serif; }}
  body {{ margin: 0; background: #0b0d12; color: #e6e8ee; line-height: 1.5; }}
  main {{ max-width: 520px; margin: 0 auto; padding: 24px 20px 48px; }}
  h1 {{ font-size: 20px; }}
  li {{ margin: 14px 0; }}
  .botao {{ display: inline-block; padding: 10px 16px; border-radius: 12px; background: #7c9cff; color: #0b0d12; font-weight: 600; text-decoration: none; }}
  .peq {{ opacity: 0.7; font-size: 14px; }}
  a {{ color: #7c9cff; }}
</style>
</head>
<body>
<main>
  <h1>Configurar o HTTPS neste aparelho</h1>
  <p>O vpad protege o acesso com HTTPS. Faça isto uma vez em cada celular.</p>
  <ol>
    <li><a class="botao" href="/vpad.mobileconfig">Baixar o perfil</a><br>Toque em <b>Permitir</b>.</li>
    <li>Abra <b>Ajustes</b> → <b>Perfil baixado</b> → <b>Instalar</b>.</li>
    <li>Abra <b>Ajustes</b> → <b>Geral</b> → <b>Sobre</b> → <b>Ajustes de Confiança de Certificado</b> e ative a confiança em <b>vpad</b>.</li>
    <li>{abrir}</li>
  </ol>
  <p class="peq">Android ou computador: baixe o <a href="/ca.crt">certificado</a> e instale-o como autoridade confiável.</p>
</main>
</body>
</html>
"#
    ))
}

async fn perfil(State(cfg): State<Arc<Configuracao>>) -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/x-apple-aspen-config"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"vpad.mobileconfig\""),
        ],
        perfil_ios(&cfg.ca_der, &cfg.nome_do_pc),
    )
        .into_response()
}

async fn certificado(State(cfg): State<Arc<Configuracao>>) -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/x-x509-ca-cert"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"vpad-ca.crt\""),
        ],
        cfg.ca_pem.clone(),
    )
        .into_response()
}

pub fn rotas(cfg: Arc<Configuracao>) -> Router {
    Router::new()
        .route("/", get(pagina))
        .route("/vpad.mobileconfig", get(perfil))
        .route("/ca.crt", get(certificado))
        .with_state(cfg)
}

#[cfg(test)]
mod testes {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use base64::{Engine, engine::general_purpose::STANDARD};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    const DER: &[u8] = &[0x30, 0x82, 0x01, 0x0a, 0xde, 0xad, 0xbe, 0xef];

    fn app() -> Router {
        rotas(Arc::new(Configuracao {
            ca_der: DER.to_vec(),
            ca_pem: "-----BEGIN CERTIFICATE-----\nZmFrZQ==\n-----END CERTIFICATE-----\n".into(),
            nome_do_pc: "meu-pc".into(),
            porta_https: 8443,
        }))
    }

    async fn pegar(metodo: &str, uri: &str, host: Option<&str>) -> (StatusCode, header::HeaderMap, String) {
        let mut req = Request::builder().method(metodo).uri(uri);
        if let Some(h) = host {
            req = req.header(header::HOST, h);
        }
        let res = app().oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
        let (status, headers) = (res.status(), res.headers().clone());
        let corpo = res.into_body().collect().await.unwrap().to_bytes();
        (status, headers, String::from_utf8_lossy(&corpo).into_owned())
    }

    #[tokio::test]
    async fn a_pagina_inicial_explica_os_passos_e_aponta_o_https_do_mesmo_endereco() {
        let (status, headers, html) = pegar("GET", "/", Some("192.168.68.103:8080")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[header::CONTENT_TYPE], "text/html; charset=utf-8");
        assert!(html.contains("/vpad.mobileconfig"), "{html}");
        assert!(html.contains("Ajustes de Confiança"), "o passo manual que o iOS exige");
        assert!(html.contains("https://192.168.68.103:8443/"), "{html}");
    }

    #[tokio::test]
    async fn funciona_com_nome_de_host_e_sem_porta() {
        let (_, _, html) = pegar("GET", "/", Some("meupc.local:8080")).await;
        assert!(html.contains("https://meupc.local:8443/"));
        let (_, _, html) = pegar("GET", "/", Some("192.168.1.9")).await;
        assert!(html.contains("https://192.168.1.9:8443/"));
    }

    #[tokio::test]
    async fn host_malicioso_nao_vira_html_nem_link() {
        for host in ["\"><script>alert(1)</script>", "x'onload='alert(1)", "evil.com/path?x=1", "a b"] {
            let (status, _, html) = pegar("GET", "/", Some(host)).await;
            assert_eq!(status, StatusCode::OK, "{host}");
            assert!(!html.contains("<script>alert"), "{host}");
            assert!(!html.contains("onload="), "{host}");
            assert!(!html.contains("evil.com"), "{host}");
        }
    }

    #[tokio::test]
    async fn sem_cabecalho_host_a_pagina_ainda_abre() {
        let (status, _, html) = pegar("GET", "/", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains("/vpad.mobileconfig"));
    }

    #[tokio::test]
    async fn o_perfil_do_iphone_e_entregue_para_instalar() {
        let (status, headers, corpo) = pegar("GET", "/vpad.mobileconfig", Some("x:8080")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[header::CONTENT_TYPE], "application/x-apple-aspen-config");
        assert!(headers[header::CONTENT_DISPOSITION].to_str().unwrap().contains("vpad.mobileconfig"));
        assert!(corpo.contains("com.apple.security.root"));
        assert!(corpo.contains(&STANDARD.encode(DER)));
    }

    #[tokio::test]
    async fn o_certificado_da_autoridade_tambem_esta_disponivel() {
        let (status, headers, corpo) = pegar("GET", "/ca.crt", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[header::CONTENT_TYPE], "application/x-x509-ca-cert");
        assert!(corpo.starts_with("-----BEGIN CERTIFICATE-----"));
    }

    #[tokio::test]
    async fn nada_do_controle_e_servido_por_http() {
        for caminho in [
            "/ws",
            "/ws?t=0123456789abcdef0123456789abcdef",
            "/api/parear",
            "/api/teclado",
            "/manifest.webmanifest",
            "/index.html",
            "/assets/index.js",
            "/qualquer-coisa",
            "/vpad.mobileconfig/../ws",
        ] {
            let (status, _, _) = pegar("GET", caminho, Some("x:8080")).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{caminho}");
        }
    }

    #[tokio::test]
    async fn o_pareamento_nao_funciona_por_http() {
        let corpo = r#"{"codigo":"123456"}"#;
        let req = Request::builder()
            .method("POST")
            .uri("/api/parear")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(corpo))
            .unwrap();
        let res = app().oneshot(req).await.unwrap();
        assert!(res.status() == StatusCode::NOT_FOUND || res.status() == StatusCode::METHOD_NOT_ALLOWED);
    }
}
