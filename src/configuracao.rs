//! Página de configuração, em HTTP simples: só entrega a autoridade do vpad (para o celular
//! passar a confiar no HTTPS) e as instruções. Nada do controle é servido por aqui: o
//! WebSocket, o pareamento e as teclas existem apenas no HTTPS.

use std::{
    future::IntoFuture,
    io,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, Uri, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};

use tokio::{
    sync::{oneshot, watch},
    time::Instant,
};

use crate::idiomas::{Idioma, TODOS, escolher, pagina as pagina_em};
use crate::mobileconfig::perfil_ios_em;

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

/// O valor de `?lang=` na consulta da URL, se houver. Só serve para escolher um dos idiomas
/// conhecidos (`Idioma::por_etiqueta` recusa o resto), então não precisa decodificar nada.
fn lang_da_consulta(consulta: Option<&str>) -> Option<&str> {
    consulta?.split('&').find_map(|par| par.strip_prefix("lang="))
}

fn idioma_do_pedido(consulta: Option<&str>, headers: &HeaderMap) -> Idioma {
    let aceito = headers.get(header::ACCEPT_LANGUAGE).and_then(|v| v.to_str().ok());
    escolher(lang_da_consulta(consulta), aceito)
}

/// A resposta depende do idioma pedido: declara qual foi usado e que ela varia com o cabeçalho.
fn com_idioma(idioma: Idioma, corpo: impl IntoResponse) -> Response {
    let mut resposta = corpo.into_response();
    let h = resposta.headers_mut();
    h.insert(header::CONTENT_LANGUAGE, axum::http::HeaderValue::from_static(idioma.codigo()));
    h.insert(header::VARY, axum::http::HeaderValue::from_static("Accept-Language"));
    resposta
}

/// Os links para trocar de idioma: o atual aparece em negrito, sem link.
fn seletor_de_idiomas(atual: Idioma) -> String {
    TODOS
        .iter()
        .map(|&i| {
            let (codigo, nome) = (i.codigo(), i.nome());
            if i == atual {
                format!(r#"<b lang="{codigo}" aria-current="true">{nome}</b>"#)
            } else {
                format!(r#"<a href="/?lang={codigo}" lang="{codigo}">{nome}</a>"#)
            }
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

async fn pagina(State(cfg): State<Arc<Configuracao>>, uri: Uri, headers: HeaderMap) -> Response {
    let idioma = idioma_do_pedido(uri.query(), &headers);
    let t = pagina_em(idioma);
    let codigo = idioma.codigo();
    let host = headers.get(header::HOST).and_then(|h| h.to_str().ok()).and_then(endereco_seguro);
    let abrir = match host {
        Some(h) => format!(r#"<a class="botao" href="https://{h}:{}/">{}</a>"#, cfg.porta_https, t.abrir_controle),
        None => t.abrir_sem_endereco_html.replace("{porta}", &cfg.porta_https.to_string()),
    };
    let idiomas = seletor_de_idiomas(idioma);
    com_idioma(
        idioma,
        Html(format!(
            r#"<!doctype html>
<html lang="{codigo}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{titulo}</title>
<style>
  :root {{ color-scheme: dark; font-family: -apple-system, system-ui, sans-serif; }}
  body {{ margin: 0; background: #0b0d12; color: #e6e8ee; line-height: 1.5; }}
  main {{ max-width: 520px; margin: 0 auto; padding: 24px 20px 48px; }}
  h1 {{ font-size: 20px; }}
  li {{ margin: 14px 0; }}
  .botao {{ display: inline-block; padding: 10px 16px; border-radius: 12px; background: #7c9cff; color: #0b0d12; font-weight: 600; text-decoration: none; }}
  .peq {{ opacity: 0.7; font-size: 14px; }}
  .idiomas {{ font-size: 14px; opacity: 0.85; }}
  .idiomas a, .idiomas b {{ white-space: nowrap; }}
  a {{ color: #7c9cff; }}
</style>
</head>
<body>
<main>
  <p class="idiomas">{rotulo_idioma}: {idiomas}</p>
  <h1>{cabecalho}</h1>
  <p>{introducao}</p>
  <ol>
    <li><a class="botao" href="/vpad.mobileconfig?lang={codigo}">{baixar}</a><br>{permitir}</li>
    <li>{instalar}</li>
    <li>{confianca}</li>
    <li>{abrir}</li>
  </ol>
  <p class="peq">{outros}</p>
</main>
</body>
</html>
"#,
            titulo = t.titulo,
            cabecalho = t.cabecalho,
            introducao = t.introducao,
            baixar = t.baixar_perfil,
            permitir = t.passo_permitir_html,
            instalar = t.passo_instalar_html,
            confianca = t.passo_confianca_html,
            outros = t.outros_sistemas_html,
            rotulo_idioma = t.idioma,
        )),
    )
}

async fn perfil(State(cfg): State<Arc<Configuracao>>, uri: Uri, headers: HeaderMap) -> Response {
    let idioma = idioma_do_pedido(uri.query(), &headers);
    com_idioma(
        idioma,
        (
            [
                (header::CONTENT_TYPE, "application/x-apple-aspen-config"),
                (header::CONTENT_DISPOSITION, "attachment; filename=\"vpad.mobileconfig\""),
            ],
            perfil_ios_em(&cfg.ca_der, &cfg.nome_do_pc, idioma),
        ),
    )
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
        .layer(axum::middleware::map_response(cabecalhos_de_seguranca))
}

/// Esta página é HTTP simples e não guarda segredo, mas ainda assim não deve ser embutida em
/// quadro nenhum, nem carregar recursos de fora.
async fn cabecalhos_de_seguranca(mut resposta: Response) -> Response {
    use axum::http::HeaderValue;

    let h = resposta.headers_mut();
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; style-src 'unsafe-inline'; img-src 'self'; base-uri 'none'; \
             form-action 'none'; frame-ancestors 'none'",
        ),
    );
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    h.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    resposta
}

struct Aberto {
    /// instante em que fecha; abrir de novo só o empurra para frente
    fim: watch::Sender<Instant>,
    endereco: SocketAddr,
    parar: Option<oneshot::Sender<()>>,
    /// distingue esta abertura de outra, feita logo depois de fechar
    geracao: u64,
}

#[derive(Default)]
struct Estado {
    aberto: Option<Aberto>,
    proxima_geracao: u64,
}

/// A página de configuração só existe enquanto o dono do PC a deixa aberta: ela entrega a
/// autoridade por HTTP simples, e esse é o momento mais exposto da instalação. Fora disso, a
/// porta fica fechada.
pub struct ServidorDeConfiguracao {
    endereco: SocketAddr,
    cfg: Arc<Configuracao>,
    estado: Mutex<Estado>,
}

impl ServidorDeConfiguracao {
    pub fn novo(endereco: SocketAddr, cfg: Arc<Configuracao>) -> Arc<Self> {
        Arc::new(Self { endereco, cfg, estado: Mutex::new(Estado::default()) })
    }

    /// Abre (ou renova o prazo de) a página por `duracao`. Devolve o endereço em que escuta.
    pub fn abrir(self: &Arc<Self>, duracao: Duration) -> io::Result<SocketAddr> {
        let mut estado = self.estado.lock().unwrap();
        if let Some(aberto) = estado.aberto.as_ref() {
            aberto.fim.send_replace(Instant::now() + duracao);
            return Ok(aberto.endereco);
        }

        let std_listener = std::net::TcpListener::bind(self.endereco)?;
        std_listener.set_nonblocking(true)?;
        let endereco = std_listener.local_addr()?;
        let listener = tokio::net::TcpListener::from_std(std_listener)?;

        let (fim, mut observa_o_fim) = watch::channel(Instant::now() + duracao);
        let (parar, recebe_parar) = oneshot::channel::<()>();
        estado.proxima_geracao += 1;
        let geracao = estado.proxima_geracao;
        estado.aberto = Some(Aberto { fim, endereco, parar: Some(parar), geracao });

        let servidor = self.clone();
        let rotas = rotas(self.cfg.clone());
        tokio::spawn(async move {
            let servir = axum::serve(listener, rotas).with_graceful_shutdown(async {
                let _ = recebe_parar.await;
            });
            // o prazo pode ser renovado enquanto espera: relê o instante a cada mudança
            let prazo = async {
                loop {
                    let fim = *observa_o_fim.borrow();
                    tokio::select! {
                        _ = tokio::time::sleep_until(fim) => {
                            if *observa_o_fim.borrow() <= Instant::now() {
                                return;
                            }
                        }
                        _ = observa_o_fim.changed() => {}
                    }
                }
            };
            let mut servir = std::pin::pin!(servir.into_future());
            tokio::select! {
                _ = &mut servir => {}
                _ = prazo => {
                    // pede o encerramento e dá um instante para as conexões em andamento acabarem
                    servidor.fechar();
                    let _ = tokio::time::timeout(Duration::from_secs(2), &mut servir).await;
                }
            }
            // só limpa o estado se ainda for esta a abertura (pode ter sido fechada e reaberta)
            let mut estado = servidor.estado.lock().unwrap();
            if estado.aberto.as_ref().is_some_and(|a| a.geracao == geracao) {
                estado.aberto = None;
            }
        });
        Ok(endereco)
    }

    pub fn aberto(&self) -> bool {
        self.estado.lock().unwrap().aberto.is_some()
    }

    pub fn segundos_restantes(&self) -> u64 {
        self.estado.lock().unwrap().aberto.as_ref().map_or(0, |a| {
            let fim = *a.fim.borrow();
            fim.saturating_duration_since(Instant::now()).as_secs_f64().ceil() as u64
        })
    }

    /// Fecha agora, sem esperar o prazo. O estado já vale como fechado ao voltar daqui; a
    /// porta é liberada em seguida, quando a tarefa do servidor termina.
    pub fn fechar(&self) {
        if let Some(mut aberto) = self.estado.lock().unwrap().aberto.take() {
            if let Some(parar) = aberto.parar.take() {
                let _ = parar.send(());
            }
        }
    }
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
    async fn a_pagina_manda_conferir_a_impressao_digital_no_terminal_e_nao_nela_mesma() {
        let (_, _, html) = pegar("GET", "/", Some("192.168.68.103:8080")).await;
        let texto = html.to_lowercase();
        assert!(texto.contains("impressão digital"), "{html}");
        assert!(texto.contains("terminal do pc"), "{html}");
        // uma página servida por HTTP pode ser trocada por quem intercepta: o valor certo só existe no terminal
        assert!(texto.contains("não confie") || texto.contains("nunca confie"), "{html}");
        assert!(!html.contains("E3:B0:C4"), "a página não exibe a impressão digital");
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

    // ---------- servidor sob demanda e temporizado ----------

    use std::{net::SocketAddr, time::Duration};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };

    fn servidor() -> Arc<ServidorDeConfiguracao> {
        ServidorDeConfiguracao::novo(
            "127.0.0.1:0".parse().unwrap(),
            Arc::new(Configuracao {
                ca_der: DER.to_vec(),
                ca_pem: "-----BEGIN CERTIFICATE-----\nZmFrZQ==\n-----END CERTIFICATE-----\n".into(),
                nome_do_pc: "meu-pc".into(),
                porta_https: 8443,
            }),
        )
    }

    async fn get(endereco: SocketAddr, caminho: &str) -> std::io::Result<String> {
        let mut tcp = TcpStream::connect(endereco).await?;
        let req = format!("GET {caminho} HTTP/1.1\r\nHost: {endereco}\r\nConnection: close\r\n\r\n");
        tcp.write_all(req.as_bytes()).await?;
        let mut resposta = String::new();
        tcp.read_to_string(&mut resposta).await?;
        Ok(resposta)
    }

    async fn recusa_conexao(endereco: SocketAddr) -> bool {
        TcpStream::connect(endereco).await.is_err()
    }

    #[tokio::test]
    async fn comeca_fechado_e_nao_serve_nada() {
        let s = servidor();
        assert!(!s.aberto());
        assert_eq!(s.segundos_restantes(), 0);
    }

    #[tokio::test]
    async fn abrir_serve_a_pagina_de_configuracao() {
        let s = servidor();
        let endereco = s.abrir(Duration::from_secs(5)).unwrap();
        assert!(s.aberto());
        let resposta = get(endereco, "/").await.unwrap();
        assert!(resposta.starts_with("HTTP/1.1 200"), "{resposta}");
        assert!(resposta.contains("/vpad.mobileconfig"));
        s.fechar();
    }

    #[tokio::test]
    async fn fecha_sozinho_quando_o_tempo_acaba() {
        let s = servidor();
        let endereco = s.abrir(Duration::from_millis(250)).unwrap();
        assert!(get(endereco, "/").await.is_ok());
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert!(!s.aberto());
        assert!(recusa_conexao(endereco).await, "a porta precisa ficar fechada depois do prazo");
    }

    #[tokio::test]
    async fn fechar_fecha_na_hora() {
        let s = servidor();
        let endereco = s.abrir(Duration::from_secs(30)).unwrap();
        s.fechar();
        assert!(!s.aberto());
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(recusa_conexao(endereco).await);
    }

    #[tokio::test]
    async fn abrir_de_novo_renova_o_prazo_sem_trocar_de_porta() {
        let s = servidor();
        let primeiro = s.abrir(Duration::from_millis(400)).unwrap();
        tokio::time::sleep(Duration::from_millis(250)).await;
        let segundo = s.abrir(Duration::from_millis(500)).unwrap();
        assert_eq!(primeiro, segundo);

        tokio::time::sleep(Duration::from_millis(350)).await; // passou do prazo original (400 ms)
        assert!(s.aberto());
        assert!(get(segundo, "/").await.is_ok());

        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(!s.aberto());
    }

    #[tokio::test]
    async fn pode_abrir_outra_vez_depois_de_fechar() {
        let s = servidor();
        let a = s.abrir(Duration::from_secs(5)).unwrap();
        s.fechar();
        tokio::time::sleep(Duration::from_millis(150)).await;
        let b = s.abrir(Duration::from_secs(5)).unwrap();
        assert!(get(b, "/").await.is_ok(), "{a} -> {b}");
        s.fechar();
    }

    #[tokio::test]
    async fn porta_ocupada_devolve_erro_e_continua_fechado() {
        let ocupada = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let s = ServidorDeConfiguracao::novo(
            ocupada.local_addr().unwrap(),
            Arc::new(Configuracao { ca_der: vec![], ca_pem: String::new(), nome_do_pc: "x".into(), porta_https: 8443 }),
        );
        assert!(s.abrir(Duration::from_secs(5)).is_err());
        assert!(!s.aberto());
    }

    #[tokio::test]
    async fn toda_resposta_traz_cabecalhos_de_seguranca() {
        for caminho in ["/", "/vpad.mobileconfig", "/ca.crt", "/nao-existe"] {
            let (_, headers, _) = pegar("GET", caminho, Some("192.168.68.103:8080")).await;
            let csp = headers[header::CONTENT_SECURITY_POLICY].to_str().unwrap();
            assert!(csp.contains("default-src 'none'"), "{caminho}: {csp}");
            assert!(csp.contains("frame-ancestors 'none'"), "{caminho}: {csp}");
            assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff", "{caminho}");
            assert_eq!(headers[header::X_FRAME_OPTIONS], "DENY", "{caminho}");
            assert_eq!(headers[header::REFERRER_POLICY], "no-referrer", "{caminho}");
            assert!(!headers.contains_key(header::STRICT_TRANSPORT_SECURITY), "a página é HTTP simples");
        }
    }

    #[tokio::test]
    async fn informa_quanto_falta_para_fechar() {
        let s = servidor();
        s.abrir(Duration::from_secs(90)).unwrap();
        let restante = s.segundos_restantes();
        assert!((88..=90).contains(&restante), "{restante}");
        s.fechar();
        assert_eq!(s.segundos_restantes(), 0);
    }

    async fn pegar_em(uri: &str, host: Option<&str>, idioma: Option<&str>) -> (StatusCode, header::HeaderMap, String) {
        let mut req = Request::builder().method("GET").uri(uri);
        if let Some(h) = host {
            req = req.header(header::HOST, h);
        }
        if let Some(i) = idioma {
            req = req.header(header::ACCEPT_LANGUAGE, i);
        }
        let res = app().oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
        let (status, headers) = (res.status(), res.headers().clone());
        let corpo = res.into_body().collect().await.unwrap().to_bytes();
        (status, headers, String::from_utf8_lossy(&corpo).into_owned())
    }

    const CONFIANCA: [(&str, &str, &str); 6] = [
        ("pt-BR", "Ajustes de Confiança de Certificado", "Configurar o HTTPS"),
        ("en", "Certificate Trust Settings", "Set up HTTPS"),
        ("es", "Ajustes de confianza de certificados", "Configurar HTTPS"),
        ("zh-CN", "证书信任设置", "设置 HTTPS"),
        ("ja", "証明書信頼設定", "HTTPS を設定"),
        ("ko", "인증서 신뢰 설정", "HTTPS 설정"),
    ];

    #[tokio::test]
    async fn sem_idioma_nenhum_a_pagina_continua_em_portugues() {
        let (_, _, html) = pegar_em("/", Some("pc.local:8080"), None).await;
        assert!(html.contains(r#"<html lang="pt-BR">"#), "{html}");
        assert!(html.contains("Ajustes de Confiança de Certificado"));
    }

    #[tokio::test]
    async fn o_accept_language_do_iphone_escolhe_o_idioma() {
        let (_, _, html) = pegar_em("/", Some("pc.local:8080"), Some("ja-JP,ja;q=0.9,en;q=0.8")).await;
        assert!(html.contains(r#"<html lang="ja">"#), "{html}");
        assert!(html.contains("証明書信頼設定"));
        assert!(html.contains("<title>vpad · HTTPS の設定</title>"));
        assert!(!html.contains("Ajustes"), "nada do português sobra");
    }

    #[tokio::test]
    async fn a_consulta_lang_traduz_a_pagina_inteira_em_cada_idioma() {
        for (codigo, confianca, cabecalho) in CONFIANCA {
            let (status, _, html) = pegar_em(&format!("/?lang={codigo}"), Some("pc.local:8080"), None).await;
            assert_eq!(status, StatusCode::OK, "{codigo}");
            assert!(html.contains(&format!(r#"<html lang="{codigo}">"#)), "{codigo}");
            assert!(html.contains(confianca), "{codigo}: {confianca}");
            assert!(html.contains(cabecalho), "{codigo}: {cabecalho}");
        }
    }

    #[tokio::test]
    async fn a_escolha_na_pagina_vence_o_idioma_do_navegador() {
        let (_, _, html) = pegar_em("/?lang=ko", Some("pc.local:8080"), Some("en-US")).await;
        assert!(html.contains(r#"<html lang="ko">"#));
    }

    #[tokio::test]
    async fn lingua_sem_traducao_recebe_o_ingles() {
        let (_, _, html) = pegar_em("/", Some("pc.local:8080"), Some("fr-FR,de;q=0.8")).await;
        assert!(html.contains(r#"<html lang="en">"#), "{html}");
    }

    #[tokio::test]
    async fn lang_invalido_nao_aparece_na_pagina_e_cai_para_o_cabecalho() {
        let (status, _, html) = pegar_em("/?lang=%3Cscript%3Ealert(1)%3C/script%3E", Some("pc.local:8080"), Some("es")).await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains(r#"<html lang="es">"#));
        assert!(!html.contains("<script>alert"), "{html}");
        assert!(!html.contains("alert(1)"), "{html}");
    }

    #[tokio::test]
    async fn o_seletor_lista_os_seis_idiomas_e_marca_o_atual_sem_link() {
        let (_, _, html) = pegar_em("/?lang=en", Some("pc.local:8080"), None).await;
        for (codigo, nome) in [("pt-BR", "Português"), ("es", "Español"), ("zh-CN", "中文"), ("ja", "日本語"), ("ko", "한국어")] {
            assert!(html.contains(&format!(r#"<a href="/?lang={codigo}" lang="{codigo}">{nome}</a>"#)), "{codigo}: {html}");
        }
        assert!(html.contains(r#"<b lang="en" aria-current="true">English</b>"#), "{html}");
        assert!(!html.contains(r#"href="/?lang=en""#), "o idioma atual não é link");
    }

    #[tokio::test]
    async fn o_link_do_perfil_leva_o_idioma_para_o_iphone_mostrar_a_descricao_certa() {
        let (_, _, html) = pegar_em("/?lang=ja", Some("pc.local:8080"), None).await;
        assert!(html.contains(r#"href="/vpad.mobileconfig?lang=ja""#), "{html}");
        let (_, _, pt) = pegar_em("/", Some("pc.local:8080"), None).await;
        assert!(pt.contains(r#"href="/vpad.mobileconfig?lang=pt-BR""#), "{pt}");
    }

    #[tokio::test]
    async fn a_resposta_declara_o_idioma_e_que_ela_varia_com_ele() {
        let (_, headers, _) = pegar_em("/", Some("pc.local:8080"), Some("ko")).await;
        assert_eq!(headers[header::CONTENT_LANGUAGE], "ko");
        assert!(headers[header::VARY].to_str().unwrap().to_ascii_lowercase().contains("accept-language"));
    }

    #[tokio::test]
    async fn o_botao_de_abrir_o_controle_e_a_dica_sem_endereco_tambem_sao_traduzidos() {
        let (_, _, com_host) = pegar_em("/?lang=en", Some("192.168.68.103:8080"), None).await;
        assert!(com_host.contains("Open the controller"), "{com_host}");
        assert!(com_host.contains("https://192.168.68.103:8443/"), "{com_host}");
        let (_, _, sem_host) = pegar_em("/?lang=en", None, None).await;
        assert!(sem_host.contains("port <b>8443</b>"), "{sem_host}");
        assert!(!sem_host.contains("{porta}"), "{sem_host}");
    }

    #[tokio::test]
    async fn o_perfil_para_o_iphone_vem_no_idioma_pedido() {
        let (_, headers, ja) = pegar_em("/vpad.mobileconfig?lang=ja", None, None).await;
        assert!(ja.contains("家庭内ネットワーク"), "{ja}");
        assert!(headers[header::VARY].to_str().unwrap().to_ascii_lowercase().contains("accept-language"));
        let (_, _, pt) = pegar_em("/vpad.mobileconfig", None, None).await;
        assert!(pt.contains("Autoridade de certificação local do vpad"), "{pt}");
        let (_, _, en) = pegar_em("/vpad.mobileconfig", None, Some("en-US")).await;
        assert!(en.contains("vpad local certificate authority"), "{en}");
    }

    #[tokio::test]
    async fn o_endereco_malicioso_continua_sem_virar_html_em_qualquer_idioma() {
        for codigo in ["pt-BR", "en", "ja"] {
            let (status, _, html) =
                pegar_em(&format!("/?lang={codigo}"), Some("evil.com/\"><script>alert(1)</script>"), None).await;
            assert_eq!(status, StatusCode::OK, "{codigo}");
            assert!(!html.contains("<script>alert"), "{codigo}");
            assert!(!html.contains("evil.com"), "{codigo}");
        }
    }

    #[tokio::test]
    async fn o_nome_de_um_idioma_nunca_quebra_no_meio() {
        let (_, _, html) = pegar_em("/?lang=en", Some("pc.local:8080"), None).await;
        assert!(html.contains(".idiomas a, .idiomas b { white-space: nowrap; }"), "{html}");
    }
}
