use std::{fs, net::IpAddr, sync::Arc, time::Duration};

use futures_util::StreamExt;
use rustls::{
    ClientConfig, RootCertStore,
    pki_types::{CertificateDer, ServerName},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_rustls::TlsConnector;
use tokio_tungstenite::{Connector, connect_async_tls_with_config};
use vpad::{
    acesso::Acesso, alerta::Alerta, armazem::ArmazemMapa, dispositivos::Dispositivos, https::servir,
    pareamento::Pareamento, tls,
};

struct Servidor {
    porta: u16,
    ca_der: Vec<u8>,
    pasta: std::path::PathBuf,
}

impl Drop for Servidor {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.pasta.parent().unwrap());
    }
}

async fn servidor(nome: &str) -> Servidor {
    let pasta = std::env::temp_dir().join(format!("vpad-https-{}-{nome}", std::process::id())).join("tls");
    let _ = fs::remove_dir_all(pasta.parent().unwrap());
    let agora = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let material = tls::carregar_ou_criar(&pasta, &["127.0.0.1".parse::<IpAddr>().unwrap()], agora).unwrap();
    let config = tls::config_do_servidor(&material).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let porta = listener.local_addr().unwrap().port();
    tokio::spawn(servir(listener, config, vpad::app()));
    Servidor { porta, ca_der: material.ca_der, pasta }
}

fn cliente(ca: Option<&[u8]>, alpn: &[&[u8]]) -> Arc<ClientConfig> {
    let mut raizes = RootCertStore::empty();
    if let Some(der) = ca {
        raizes.add(CertificateDer::from(der.to_vec())).unwrap();
    }
    let mut config = ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(raizes)
        .with_no_client_auth();
    config.alpn_protocols = alpn.iter().map(|a| a.to_vec()).collect();
    Arc::new(config)
}

async fn conectar(s: &Servidor, config: Arc<ClientConfig>) -> std::io::Result<tokio_rustls::client::TlsStream<TcpStream>> {
    let tcp = TcpStream::connect(("127.0.0.1", s.porta)).await?;
    let nome = ServerName::IpAddress("127.0.0.1".parse::<IpAddr>().unwrap().into());
    TlsConnector::from(config).connect(nome, tcp).await
}

async fn pedir(s: &Servidor, caminho: &str) -> String {
    let mut tls = conectar(s, cliente(Some(&s.ca_der), &[b"http/1.1"])).await.unwrap();
    let req = format!("GET {caminho} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    tls.write_all(req.as_bytes()).await.unwrap();
    let mut resposta = String::new();
    let _ = tokio::time::timeout(Duration::from_secs(3), tls.read_to_string(&mut resposta)).await;
    resposta
}

#[tokio::test]
async fn serve_a_pagina_por_https_para_quem_confia_na_autoridade() {
    let s = servidor("pagina").await;
    let resposta = pedir(&s, "/manifest.webmanifest").await;
    assert!(resposta.starts_with("HTTP/1.1 200"), "{resposta}");
    assert!(resposta.to_lowercase().contains("application/manifest+json"), "{resposta}");
}

#[tokio::test]
async fn o_websocket_funciona_sobre_tls_e_recusa_quem_nao_tem_token() {
    let s = servidor("wss").await;
    let connector = Connector::Rustls(cliente(Some(&s.ca_der), &[]));
    let url = format!("wss://127.0.0.1:{}/ws", s.porta);
    let (mut ws, _) = connect_async_tls_with_config(url, None, false, Some(connector)).await.unwrap();

    let primeira = tokio::time::timeout(Duration::from_secs(2), ws.next()).await.unwrap().unwrap().unwrap();
    assert_eq!(primeira.into_data().to_vec(), vec![5, 1], "sem token: recusado, mas o upgrade sobre TLS funcionou");
}

#[tokio::test]
async fn negocia_http2_quando_o_cliente_oferece() {
    let s = servidor("h2").await;
    let tls = conectar(&s, cliente(Some(&s.ca_der), &[b"h2", b"http/1.1"])).await.unwrap();
    assert_eq!(tls.get_ref().1.alpn_protocol(), Some(&b"h2"[..]));
}

#[tokio::test]
async fn cliente_que_nao_confia_na_autoridade_nao_completa_a_conexao() {
    let s = servidor("sem-ca").await;
    assert!(conectar(&s, cliente(None, &[])).await.is_err());
}

#[tokio::test]
async fn um_handshake_ruim_nao_derruba_o_servidor() {
    let s = servidor("resiliente").await;
    for _ in 0..3 {
        let _ = conectar(&s, cliente(None, &[])).await; // recusado pelo cliente
        let mut cru = TcpStream::connect(("127.0.0.1", s.porta)).await.unwrap();
        let _ = cru.write_all(b"isto nao e TLS\r\n\r\n").await; // lixo no lugar do handshake
    }
    assert!(pedir(&s, "/manifest.webmanifest").await.starts_with("HTTP/1.1 200"));
}

#[tokio::test]
async fn http_em_claro_na_porta_https_nao_serve_nada() {
    let s = servidor("em-claro").await;
    let mut cru = TcpStream::connect(("127.0.0.1", s.porta)).await.unwrap();
    cru.write_all(b"GET /manifest.webmanifest HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").await.unwrap();
    let mut resposta = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(2), cru.read_to_end(&mut resposta)).await;
    let texto = String::from_utf8_lossy(&resposta);
    assert!(!texto.contains("200 OK") && !texto.contains("manifest"), "{texto:?}");
}

#[tokio::test]
async fn atende_varias_conexoes_ao_mesmo_tempo() {
    let s = servidor("varias").await;
    let respostas = futures_util::future::join_all((0..8).map(|_| pedir(&s, "/manifest.webmanifest"))).await;
    assert!(respostas.iter().all(|r| r.starts_with("HTTP/1.1 200")));
}

#[tokio::test]
async fn o_alerta_de_pareamento_leva_o_ip_real_de_quem_conectou_por_https() {
    let pasta = std::env::temp_dir().join(format!("vpad-https-{}-ip", std::process::id())).join("tls");
    let _ = fs::remove_dir_all(pasta.parent().unwrap());
    let agora = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let material = tls::carregar_ou_criar(&pasta, &["127.0.0.1".parse::<IpAddr>().unwrap()], agora).unwrap();

    let vistos = Arc::new(std::sync::Mutex::new(vec![]));
    let destino = vistos.clone();
    let acesso = Acesso::com_alertas(
        Dispositivos::em_memoria(),
        Pareamento::com_segredo(b"segredo-de-teste-123".to_vec()),
        Duration::from_secs(6),
        Arc::new(move |a| destino.lock().unwrap().push(a)),
    );
    acesso.pareamento().abrir(agora, 600);
    let app = vpad::app_com(
        tokio::sync::mpsc::channel(64).0,
        tokio::sync::watch::channel(0).1,
        Arc::new(ArmazemMapa::em_memoria()),
        acesso,
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let porta = listener.local_addr().unwrap().port();
    tokio::spawn(servir(listener, tls::config_do_servidor(&material).unwrap(), app));

    let s = Servidor { porta, ca_der: material.ca_der.clone(), pasta: pasta.clone() };
    let mut tls = conectar(&s, cliente(Some(&s.ca_der), &[b"http/1.1"])).await.unwrap();
    let corpo = r#"{"codigo":"000000"}"#;
    let req = format!(
        "POST /api/parear HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{corpo}",
        corpo.len()
    );
    tls.write_all(req.as_bytes()).await.unwrap();
    let mut resposta = String::new();
    let _ = tokio::time::timeout(Duration::from_secs(3), tls.read_to_string(&mut resposta)).await;
    assert!(resposta.starts_with("HTTP/1.1 401"), "{resposta}");

    let alertas = vistos.lock().unwrap();
    assert_eq!(alertas.len(), 1, "{alertas:?}");
    let Alerta::CodigoErrado { origem, .. } = &alertas[0] else { panic!("{alertas:?}") };
    assert_eq!(*origem, Some("127.0.0.1".parse().unwrap()), "o IP de quem conectou chega até o alerta");
}
