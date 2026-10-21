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
    http::{HeaderMap, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};

use tokio::{
    sync::{oneshot, watch},
    time::Instant,
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
    <li>Abra <b>Ajustes</b> → <b>Perfil baixado</b>, toque no certificado e em <b>Mais detalhes</b>, e confira se a <b>impressão digital SHA-256</b> é igual à mostrada no <b>terminal do PC</b>. Só então toque em <b>Instalar</b>. <b>Não confie</b> em nenhuma impressão digital exibida por esta página: quem estiver interceptando a rede poderia trocá-la.</li>
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
}
