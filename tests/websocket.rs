use std::{sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver},
    watch,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use vpad::{acesso::Acesso, armazem::ArmazemMapa, protocolo::Evento};

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

type Cliente = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

struct Servidor {
    url: String,
    eventos: UnboundedReceiver<Evento>,
    perfil: watch::Sender<u8>,
}

async fn servidor() -> Servidor {
    servidor_com(Duration::from_secs(6)).await
}

async fn servidor_com(inatividade: Duration) -> Servidor {
    let (tx, eventos) = mpsc::unbounded_channel();
    let (perfil, perfil_rx) = watch::channel(0u8);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}/ws", listener.local_addr().unwrap());
    let app = vpad::app_com(
        tx,
        perfil_rx,
        Arc::new(ArmazemMapa::em_memoria()),
        Acesso::novo(TOKEN, inatividade),
    );
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    Servidor { url, eventos, perfil }
}

fn url_de(s: &Servidor, token: &str, id: &str) -> String {
    format!("{}?t={token}&id={id}", s.url)
}

/// Conecta como o aparelho `id` (com o token certo) e consome o aviso de perfil.
async fn conectar_como(s: &Servidor, id: &str) -> Cliente {
    let mut ws = connect_async(url_de(s, TOKEN, id)).await.unwrap().0;
    assert_eq!(proxima(&mut ws).await, vec![4, 0], "aviso inicial de perfil");
    ws
}

async fn conectar(s: &Servidor) -> Cliente {
    conectar_como(s, "celular-1").await
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

/// O servidor encerrou esta conexão?
async fn foi_encerrada(ws: &mut Cliente) -> bool {
    let fim = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < fim {
        match tokio::time::timeout(Duration::from_millis(300), ws.next()).await {
            Ok(None) | Ok(Some(Err(_))) | Ok(Some(Ok(Message::Close(_)))) => return true,
            _ => {}
        }
    }
    false
}

async fn proximo(rx: &mut UnboundedReceiver<Evento>) -> Evento {
    tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("tempo esgotado esperando evento")
        .expect("canal fechado")
}

async fn silencio(rx: &mut UnboundedReceiver<Evento>) {
    assert!(
        tokio::time::timeout(Duration::from_millis(300), rx.recv()).await.is_err(),
        "não era para chegar nenhum evento"
    );
}

fn botao(id: u8, pressionado: bool) -> Message {
    Message::Binary(vec![1, id, pressionado as u8].into())
}

// ---------- protocolo básico ----------

#[tokio::test]
async fn ao_conectar_o_servidor_informa_o_perfil_atual() {
    let s = servidor().await;
    s.perfil.send(2).unwrap();
    let mut ws = connect_async(url_de(&s, TOKEN, "celular-1")).await.unwrap().0;
    assert_eq!(proxima(&mut ws).await, vec![4, 2]);
}

#[tokio::test]
async fn ping_e_devolvido_igual() {
    let s = servidor().await;
    let mut ws = conectar(&s).await;
    let ping = vec![3, 1, 2, 3, 4, 5, 6, 7, 8];
    ws.send(Message::Binary(ping.clone().into())).await.unwrap();
    assert_eq!(proxima(&mut ws).await, ping);
}

#[tokio::test]
async fn botao_chega_como_evento() {
    let mut s = servidor().await;
    let mut ws = conectar(&s).await;
    ws.send(botao(0, true)).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::Botao { id: 0, pressionado: true });
}

#[tokio::test]
async fn mensagem_invalida_e_ignorada_e_a_conexao_continua() {
    let mut s = servidor().await;
    let mut ws = conectar(&s).await;
    ws.send(Message::Binary(vec![9, 9, 9].into())).await.unwrap();
    ws.send(Message::Text("oi".into())).await.unwrap();
    ws.send(Message::Binary(vec![4, 9].into())).await.unwrap(); // perfil inexistente
    ws.send(Message::Binary(vec![2, 0, 0x34, 0x12].into())).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::Eixo { id: 0, valor: 0x1234 });
}

#[tokio::test]
async fn pedido_de_troca_de_perfil_vira_evento() {
    let mut s = servidor().await;
    let mut ws = conectar(&s).await;
    ws.send(Message::Binary(vec![4, 1].into())).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::TrocarPerfil(1));
}

#[tokio::test]
async fn mudanca_de_perfil_chega_ao_dono_conectado() {
    let s = servidor().await;
    let mut ws = conectar(&s).await;
    s.perfil.send(1).unwrap();
    assert_eq!(proxima(&mut ws).await, vec![4, 1]);
}

#[tokio::test]
async fn reafirmar_o_mesmo_perfil_tambem_notifica() {
    // usado quando a troca falha: o celular precisa saber que continua no perfil antigo
    let s = servidor().await;
    let mut ws = conectar(&s).await;
    s.perfil.send(0).unwrap();
    assert_eq!(proxima(&mut ws).await, vec![4, 0]);
}

#[tokio::test]
async fn desconectar_solta_tudo() {
    let mut s = servidor().await;
    let mut ws = conectar(&s).await;
    ws.send(botao(0, true)).await.unwrap();
    assert!(matches!(proximo(&mut s.eventos).await, Evento::Botao { .. }));

    ws.close(None).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::SoltarTudo);
}

#[tokio::test]
async fn queda_abrupta_tambem_solta_tudo() {
    let mut s = servidor().await;
    let ws = conectar(&s).await;
    drop(ws);
    assert_eq!(proximo(&mut s.eventos).await, Evento::SoltarTudo);
}

// ---------- token ----------

#[tokio::test]
async fn sem_token_ou_com_token_errado_e_recusado_e_nada_chega_ao_controle() {
    let mut s = servidor().await;
    for url in [
        s.url.clone(),
        format!("{}?id=celular-1", s.url),
        format!("{}?t=errado&id=celular-1", s.url),
        format!("{}?t={TOKEN}x&id=celular-1", s.url),
        format!("{}?t={TOKEN}", s.url), // sem identificação do aparelho
        format!("{}?t={TOKEN}&id=", s.url),
    ] {
        let mut ws = connect_async(&url).await.unwrap().0;
        assert_eq!(proxima(&mut ws).await, vec![5, 2], "{url}");
        let _ = ws.send(botao(0, true)).await;
        assert!(foi_encerrada(&mut ws).await, "{url}");
    }
    silencio(&mut s.eventos).await;
}

#[tokio::test]
async fn recusa_por_token_nao_ocupa_a_vaga() {
    let s = servidor().await;
    let mut intruso = connect_async(format!("{}?t=errado&id=celular-1", s.url)).await.unwrap().0;
    assert_eq!(proxima(&mut intruso).await, vec![5, 2]);
    let _dono = conectar(&s).await;
}

// ---------- um aparelho por vez ----------

#[tokio::test]
async fn segundo_aparelho_e_recusado_sem_atrapalhar_o_dono() {
    let mut s = servidor().await;
    let mut dono = conectar_como(&s, "celular-1").await;
    dono.send(botao(0, true)).await.unwrap();
    assert!(matches!(proximo(&mut s.eventos).await, Evento::Botao { .. }));

    let mut intruso = connect_async(url_de(&s, TOKEN, "celular-2")).await.unwrap().0;
    assert_eq!(proxima(&mut intruso).await, vec![5, 1]);
    let _ = intruso.send(botao(1, true)).await;
    assert!(foi_encerrada(&mut intruso).await);

    // a saída do recusado não pode soltar os botões do dono, nem seus eventos passam
    silencio(&mut s.eventos).await;
    dono.send(Message::Binary(vec![2, 0, 0x34, 0x12].into())).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::Eixo { id: 0, valor: 0x1234 });
}

#[tokio::test]
async fn quando_o_dono_sai_outro_aparelho_pode_assumir() {
    let mut s = servidor().await;
    let mut dono = conectar_como(&s, "celular-1").await;
    dono.close(None).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::SoltarTudo);

    let mut novo = conectar_como(&s, "celular-2").await;
    novo.send(botao(0, true)).await.unwrap();
    assert!(matches!(proximo(&mut s.eventos).await, Evento::Botao { .. }));
}

#[tokio::test]
async fn o_mesmo_aparelho_reassume_derrubando_a_conexao_antiga() {
    let mut s = servidor().await;
    let mut antiga = conectar_como(&s, "celular-1").await;
    antiga.send(botao(0, true)).await.unwrap();
    assert!(matches!(proximo(&mut s.eventos).await, Evento::Botao { .. }));

    let mut nova = conectar_como(&s, "celular-1").await;
    assert!(foi_encerrada(&mut antiga).await, "a conexão antiga precisa cair");
    // a nova começa limpa: o botão que a antiga deixou pressionado é solto, uma vez só
    assert_eq!(proximo(&mut s.eventos).await, Evento::SoltarTudo);
    silencio(&mut s.eventos).await;

    nova.send(botao(1, true)).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::Botao { id: 1, pressionado: true });
}

#[tokio::test]
async fn dono_em_silencio_perde_a_vaga_depois_do_tempo_de_inatividade() {
    let mut s = servidor_com(Duration::from_millis(300)).await;
    let mut mudo = conectar_como(&s, "celular-1").await;

    assert!(foi_encerrada(&mut mudo).await);
    assert_eq!(proximo(&mut s.eventos).await, Evento::SoltarTudo);
    let _outro = conectar_como(&s, "celular-2").await;
}

#[tokio::test]
async fn dono_ativo_nao_perde_a_vaga() {
    let s = servidor_com(Duration::from_millis(400)).await;
    let mut dono = conectar_como(&s, "celular-1").await;

    // pings a cada 100 ms durante 1,2 s: bem mais que o tempo de inatividade
    for _ in 0..12 {
        dono.send(Message::Binary(vec![3, 0, 0, 0, 0, 0, 0, 0, 0].into())).await.unwrap();
        assert_eq!(proxima(&mut dono).await.first(), Some(&3));
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let mut intruso = connect_async(url_de(&s, TOKEN, "celular-2")).await.unwrap().0;
    assert_eq!(proxima(&mut intruso).await, vec![5, 1]);
}
