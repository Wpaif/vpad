//! Servidor HTTPS: aceita conexões TCP, faz o handshake TLS e entrega cada conexão ao axum
//! (HTTP/1.1 ou HTTP/2, com upgrade para WebSocket).

use std::{io, sync::Arc, time::Duration};

use axum::{Router, extract::ConnectInfo};
use hyper_util::{
    rt::{TokioExecutor, TokioIo, TokioTimer},
    server::conn::auto::Builder,
    service::TowerToHyperService,
};
use rustls::ServerConfig;
use tokio::{net::TcpListener, sync::Semaphore};
use tokio_rustls::TlsAcceptor;

/// Quem abre a conexão e não completa o handshake nesse tempo é largado.
const TEMPO_DO_HANDSHAKE: Duration = Duration::from_secs(10);
/// Quem conecta e não termina de mandar os cabeçalhos nesse tempo é largado.
const TEMPO_DOS_CABECALHOS: Duration = Duration::from_secs(5);
/// Teto de conexões atendidas ao mesmo tempo: um cliente (ou uma rede) não esgota o processo.
const MAXIMO_DE_CONEXOES: usize = 64;

/// Atende para sempre. Um handshake que falha (cliente que não confia na autoridade, lixo,
/// HTTP em claro) só descarta aquela conexão.
pub async fn servir(listener: TcpListener, config: Arc<ServerConfig>, app: Router) -> io::Result<()> {
    let acceptor = TlsAcceptor::from(config);
    let vagas = Arc::new(Semaphore::new(MAXIMO_DE_CONEXOES));
    loop {
        let (tcp, origem) = match listener.accept().await {
            Ok(conexao) => conexao,
            Err(e) => {
                // por exemplo, falta de descritores de arquivo: espera um pouco e segue
                eprintln!("erro ao aceitar conexão: {e}");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        let Ok(vaga) = vagas.clone().try_acquire_owned() else {
            // sem vaga: fecha na hora, sem handshake nem resposta
            drop(tcp);
            continue;
        };
        // Sem Nagle: cada toque vira um pacote imediato.
        let _ = tcp.set_nodelay(true);

        let acceptor = acceptor.clone();
        // cada requisição desta conexão leva o IP de quem conectou (usado nos alertas)
        let app = app.clone().layer(axum::Extension(ConnectInfo(origem)));
        tokio::spawn(async move {
            let _vaga = vaga;
            let Ok(Ok(tls)) = tokio::time::timeout(TEMPO_DO_HANDSHAKE, acceptor.accept(tcp)).await else {
                return;
            };
            let mut construtor = Builder::new(TokioExecutor::new());
            construtor
                .http1()
                .timer(TokioTimer::new())
                .header_read_timeout(Some(TEMPO_DOS_CABECALHOS));
            construtor.http2().timer(TokioTimer::new()).max_header_list_size(16 * 1024).keep_alive_interval(Some(Duration::from_secs(30))).keep_alive_timeout(Duration::from_secs(10));
            let _ = construtor
                .serve_connection_with_upgrades(TokioIo::new(tls), TowerToHyperService::new(app))
                .await;
        });
    }
}
