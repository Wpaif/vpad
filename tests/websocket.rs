use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use vpad::protocolo::Evento;

type Cliente = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

async fn servidor() -> (String, UnboundedReceiver<Evento>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}/ws", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, vpad::app_com(tx)).await.unwrap() });
    (url, rx)
}

async fn conectar(url: &str) -> Cliente {
    connect_async(url).await.unwrap().0
}

async fn proximo(rx: &mut UnboundedReceiver<Evento>) -> Evento {
    tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("tempo esgotado esperando evento")
        .expect("canal fechado")
}

#[tokio::test]
async fn ping_e_devolvido_igual() {
    let (url, _rx) = servidor().await;
    let mut ws = conectar(&url).await;
    let ping = vec![3, 1, 2, 3, 4, 5, 6, 7, 8];
    ws.send(Message::Binary(ping.clone().into())).await.unwrap();

    let resposta = tokio::time::timeout(Duration::from_secs(2), ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(resposta.into_data().to_vec(), ping);
}

#[tokio::test]
async fn botao_chega_como_evento() {
    let (url, mut rx) = servidor().await;
    let mut ws = conectar(&url).await;
    ws.send(Message::Binary(vec![1, 0, 1].into())).await.unwrap();

    assert_eq!(
        proximo(&mut rx).await,
        Evento::Botao { id: 0, pressionado: true }
    );
}

#[tokio::test]
async fn mensagem_invalida_e_ignorada_e_a_conexao_continua() {
    let (url, mut rx) = servidor().await;
    let mut ws = conectar(&url).await;
    ws.send(Message::Binary(vec![9, 9, 9].into())).await.unwrap();
    ws.send(Message::Text("oi".into())).await.unwrap();
    ws.send(Message::Binary(vec![2, 0, 0x34, 0x12].into())).await.unwrap();

    assert_eq!(proximo(&mut rx).await, Evento::Eixo { id: 0, valor: 0x1234 });
}

#[tokio::test]
async fn desconectar_solta_tudo() {
    let (url, mut rx) = servidor().await;
    let mut ws = conectar(&url).await;
    ws.send(Message::Binary(vec![1, 0, 1].into())).await.unwrap();
    assert!(matches!(proximo(&mut rx).await, Evento::Botao { .. }));

    ws.close(None).await.unwrap();
    assert_eq!(proximo(&mut rx).await, Evento::SoltarTudo);
}

#[tokio::test]
async fn queda_abrupta_tambem_solta_tudo() {
    let (url, mut rx) = servidor().await;
    let ws = conectar(&url).await;
    drop(ws);
    assert_eq!(proximo(&mut rx).await, Evento::SoltarTudo);
}
