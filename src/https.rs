//! Servidor HTTPS: aceita conexões TCP, faz o handshake TLS e entrega cada conexão ao axum
//! (HTTP/1.1 ou HTTP/2, com upgrade para WebSocket).

use std::{io, sync::Arc, time::Duration};

use axum::{Router, extract::ConnectInfo};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
    service::TowerToHyperService,
};
use rustls::ServerConfig;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

/// Quem abre a conexão e não completa o handshake nesse tempo é largado.
const TEMPO_DO_HANDSHAKE: Duration = Duration::from_secs(10);

/// Atende para sempre. Um handshake que falha (cliente que não confia na autoridade, lixo,
/// HTTP em claro) só descarta aquela conexão.
pub async fn servir(listener: TcpListener, config: Arc<ServerConfig>, app: Router) -> io::Result<()> {
    let acceptor = TlsAcceptor::from(config);
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
        // Sem Nagle: cada toque vira um pacote imediato.
        let _ = tcp.set_nodelay(true);

        let acceptor = acceptor.clone();
        // cada requisição desta conexão leva o IP de quem conectou (usado nos alertas)
        let app = app.clone().layer(axum::Extension(ConnectInfo(origem)));
        tokio::spawn(async move {
            let Ok(Ok(tls)) = tokio::time::timeout(TEMPO_DO_HANDSHAKE, acceptor.accept(tcp)).await else {
                return;
            };
            let _ = Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(TokioIo::new(tls), TowerToHyperService::new(app))
                .await;
        });
    }
}
