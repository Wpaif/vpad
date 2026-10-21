use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver},
    watch,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use vpad::protocolo::Evento;

type Cliente = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

struct Servidor {
    url: String,
    eventos: UnboundedReceiver<Evento>,
    perfil: watch::Sender<u8>,
}

async fn servidor() -> Servidor {
    let (tx, eventos) = mpsc::unbounded_channel();
    let (perfil, perfil_rx) = watch::channel(0u8);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}/ws", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, vpad::app_com(tx, perfil_rx)).await.unwrap()
    });
    Servidor { url, eventos, perfil }
}

/// Conecta e consome o aviso de perfil que o servidor manda ao abrir.
async fn conectar(url: &str) -> Cliente {
    let mut ws = connect_async(url).await.unwrap().0;
    assert_eq!(proxima(&mut ws).await, vec![4, 0], "aviso inicial de perfil");
    ws
}

async fn proxima(ws: &mut Cliente) -> Vec<u8> {
    tokio::time::timeout(Duration::from_secs(2), ws.next())
        .await
        .expect("tempo esgotado esperando mensagem")
        .unwrap()
        .unwrap()
        .into_data()
        .to_vec()
}

async fn proximo(rx: &mut UnboundedReceiver<Evento>) -> Evento {
    tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("tempo esgotado esperando evento")
        .expect("canal fechado")
}

#[tokio::test]
async fn ao_conectar_o_servidor_informa_o_perfil_atual() {
    let s = servidor().await;
    s.perfil.send(2).unwrap();
    let mut ws = connect_async(&s.url).await.unwrap().0;
    assert_eq!(proxima(&mut ws).await, vec![4, 2]);
}

#[tokio::test]
async fn ping_e_devolvido_igual() {
    let s = servidor().await;
    let mut ws = conectar(&s.url).await;
    let ping = vec![3, 1, 2, 3, 4, 5, 6, 7, 8];
    ws.send(Message::Binary(ping.clone().into())).await.unwrap();
    assert_eq!(proxima(&mut ws).await, ping);
}

#[tokio::test]
async fn botao_chega_como_evento() {
    let mut s = servidor().await;
    let mut ws = conectar(&s.url).await;
    ws.send(Message::Binary(vec![1, 0, 1].into())).await.unwrap();

    assert_eq!(
        proximo(&mut s.eventos).await,
        Evento::Botao { id: 0, pressionado: true }
    );
}

#[tokio::test]
async fn mensagem_invalida_e_ignorada_e_a_conexao_continua() {
    let mut s = servidor().await;
    let mut ws = conectar(&s.url).await;
    ws.send(Message::Binary(vec![9, 9, 9].into())).await.unwrap();
    ws.send(Message::Text("oi".into())).await.unwrap();
    ws.send(Message::Binary(vec![4, 9].into())).await.unwrap(); // perfil inexistente
    ws.send(Message::Binary(vec![2, 0, 0x34, 0x12].into())).await.unwrap();

    assert_eq!(proximo(&mut s.eventos).await, Evento::Eixo { id: 0, valor: 0x1234 });
}

#[tokio::test]
async fn pedido_de_troca_de_perfil_vira_evento() {
    let mut s = servidor().await;
    let mut ws = conectar(&s.url).await;
    ws.send(Message::Binary(vec![4, 1].into())).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::TrocarPerfil(1));
}

#[tokio::test]
async fn mudanca_de_perfil_chega_a_todos_os_celulares_conectados() {
    let s = servidor().await;
    let mut a = conectar(&s.url).await;
    let mut b = conectar(&s.url).await;

    s.perfil.send(1).unwrap();

    assert_eq!(proxima(&mut a).await, vec![4, 1]);
    assert_eq!(proxima(&mut b).await, vec![4, 1]);
}

#[tokio::test]
async fn reafirmar_o_mesmo_perfil_tambem_notifica() {
    // usado quando a troca falha: o celular precisa saber que continua no perfil antigo
    let s = servidor().await;
    let mut ws = conectar(&s.url).await;
    s.perfil.send(0).unwrap();
    assert_eq!(proxima(&mut ws).await, vec![4, 0]);
}

#[tokio::test]
async fn desconectar_solta_tudo() {
    let mut s = servidor().await;
    let mut ws = conectar(&s.url).await;
    ws.send(Message::Binary(vec![1, 0, 1].into())).await.unwrap();
    assert!(matches!(proximo(&mut s.eventos).await, Evento::Botao { .. }));

    ws.close(None).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::SoltarTudo);
}

#[tokio::test]
async fn queda_abrupta_tambem_solta_tudo() {
    let mut s = servidor().await;
    let ws = conectar(&s.url).await;
    drop(ws);
    assert_eq!(proximo(&mut s.eventos).await, Evento::SoltarTudo);
}
