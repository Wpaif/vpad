use std::{sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{
    mpsc::{self, Receiver},
    watch,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use vpad::{
    acesso::Acesso,
    aprovacao::{Decisao, Pedido, Tipo},
    armazem::ArmazemMapa,
    dispositivos::Dispositivos,
    pareamento::Pareamento,
    protocolo::Evento,
};

type Cliente = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

struct Servidor {
    url: String,
    acesso: Arc<Acesso>,
    eventos: Receiver<Evento>,
    perfil: watch::Sender<u8>,
}

async fn servidor() -> Servidor {
    servidor_com(Duration::from_secs(6)).await
}

async fn servidor_com(inatividade: Duration) -> Servidor {
    servidor_completo(inatividade, Duration::from_secs(30)).await
}

async fn servidor_completo(inatividade: Duration, prazo_da_aprovacao: Duration) -> Servidor {
    let (tx, eventos) = mpsc::channel(64);
    let (perfil, perfil_rx) = watch::channel(0u8);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}/ws", listener.local_addr().unwrap());
    let acesso = Acesso::com_opcoes(
        Dispositivos::em_memoria(),
        Pareamento::com_segredo(b"segredo-de-teste-123".to_vec()),
        inatividade,
        vpad::alerta::ignorar(),
        prazo_da_aprovacao,
    );
    let app = vpad::app_com(tx, perfil_rx, Arc::new(ArmazemMapa::em_memoria()), acesso.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    Servidor { url, acesso, eventos, perfil }
}

/// Pareia um aparelho novo e devolve (id, token).
fn parear(s: &Servidor, nome: &str) -> (String, String) {
    let p = s.acesso.dispositivos().parear(nome, vpad::dispositivos::agora()).unwrap();
    (p.id, p.token)
}

/// Pareia um aparelho que o dono do PC já autorizou a usar o modo teclado.
fn parear_teclado(s: &Servidor, nome: &str) -> (String, String) {
    let p = s.acesso.dispositivos().parear_com(nome, vpad::dispositivos::agora(), true).unwrap();
    (p.id, p.token)
}

/// Espera o pedido de aprovação aparecer no "terminal" e devolve o id dele.
async fn pedido_pendente(s: &Servidor) -> Pedido {
    for _ in 0..400 {
        if let Some(p) = s.acesso.aprovacoes().pendente() {
            return p;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("nenhum pedido de aprovação chegou ao terminal");
}

fn url_com(s: &Servidor, token: &str) -> String {
    format!("{}?t={token}", s.url)
}

/// Conecta com `token` e consome o aviso de perfil.
async fn conectar(s: &Servidor, token: &str) -> Cliente {
    let mut ws = connect_async(url_com(s, token)).await.unwrap().0;
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

async fn proximo(rx: &mut Receiver<Evento>) -> Evento {
    tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("tempo esgotado esperando evento")
        .expect("canal fechado")
}

async fn silencio(rx: &mut Receiver<Evento>) {
    assert!(
        tokio::time::timeout(Duration::from_millis(300), rx.recv()).await.is_err(),
        "não era para chegar nenhum evento"
    );
}

fn botao(id: u8, pressionado: bool) -> Message {
    Message::Binary(vec![1, id, pressionado as u8].into())
}

fn ping() -> Message {
    Message::Binary(vec![3, 1, 2, 3, 4, 5, 6, 7, 8].into())
}

// ---------- protocolo básico ----------

#[tokio::test]
async fn ao_conectar_o_servidor_informa_o_perfil_atual() {
    let s = servidor().await;
    let (_, token) = parear(&s, "iPhone");
    s.perfil.send(2).unwrap();
    let mut ws = connect_async(url_com(&s, &token)).await.unwrap().0;
    assert_eq!(proxima(&mut ws).await, vec![4, 2]);
}

#[tokio::test]
async fn ping_e_devolvido_igual() {
    let s = servidor().await;
    let (_, token) = parear(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;
    ws.send(ping()).await.unwrap();
    assert_eq!(proxima(&mut ws).await, vec![3, 1, 2, 3, 4, 5, 6, 7, 8]);
}

#[tokio::test]
async fn botao_chega_como_evento() {
    let mut s = servidor().await;
    let (_, token) = parear(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;
    ws.send(botao(0, true)).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::Botao { id: 0, pressionado: true });
}

#[tokio::test]
async fn mensagem_invalida_e_ignorada_e_a_conexao_continua() {
    let mut s = servidor().await;
    let (_, token) = parear(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;
    ws.send(Message::Binary(vec![9, 9, 9].into())).await.unwrap();
    ws.send(Message::Text("oi".into())).await.unwrap();
    ws.send(Message::Binary(vec![4, 9].into())).await.unwrap(); // perfil inexistente
    ws.send(Message::Binary(vec![2, 0, 0x34, 0x12].into())).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::Eixo { id: 0, valor: 0x1234 });
}

#[tokio::test]
async fn pedido_de_troca_de_perfil_vira_evento() {
    let mut s = servidor().await;
    let (_, token) = parear(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;
    ws.send(Message::Binary(vec![4, 1].into())).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::TrocarPerfil(1));
}

#[tokio::test]
async fn desconectar_solta_tudo() {
    let mut s = servidor().await;
    let (_, token) = parear(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;
    ws.send(botao(0, true)).await.unwrap();
    assert!(matches!(proximo(&mut s.eventos).await, Evento::Botao { .. }));

    ws.close(None).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::SoltarTudo);
}

#[tokio::test]
async fn queda_abrupta_tambem_solta_tudo() {
    let mut s = servidor().await;
    let (_, token) = parear(&s, "iPhone");
    let ws = conectar(&s, &token).await;
    drop(ws);
    assert_eq!(proximo(&mut s.eventos).await, Evento::SoltarTudo);
}

// ---------- token por dispositivo ----------

#[tokio::test]
async fn sem_token_ou_com_token_invalido_e_recusado_e_nada_chega_ao_controle() {
    let mut s = servidor().await;
    let (_, valido) = parear(&s, "iPhone");
    for url in [
        s.url.clone(),
        format!("{}?t=", s.url),
        format!("{}?t=errado", s.url),
        format!("{}?t={valido}x", s.url),
        format!("{}?t={}", s.url, &valido[..31]),
        format!("{}?id=celular-1", s.url),
    ] {
        let mut ws = connect_async(&url).await.unwrap().0;
        assert_eq!(proxima(&mut ws).await, vec![5, 1], "{url}");
        let _ = ws.send(botao(0, true)).await;
        assert!(foi_encerrada(&mut ws).await, "{url}");
    }
    silencio(&mut s.eventos).await;
}

#[tokio::test]
async fn ninguem_pareado_significa_ninguem_conecta() {
    let s = servidor().await;
    let mut ws = connect_async(url_com(&s, "0123456789abcdef0123456789abcdef")).await.unwrap().0;
    assert_eq!(proxima(&mut ws).await, vec![5, 1]);
}

#[tokio::test]
async fn varios_aparelhos_pareados_conectam_ao_mesmo_tempo() {
    let mut s = servidor().await;
    let (_, token_a) = parear(&s, "iPhone");
    let (_, token_b) = parear(&s, "iPad");
    let mut a = conectar(&s, &token_a).await;
    let mut b = conectar(&s, &token_b).await;

    a.send(botao(0, true)).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::Botao { id: 0, pressionado: true });
    b.send(botao(1, true)).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::Botao { id: 1, pressionado: true });
}

#[tokio::test]
async fn mudanca_de_perfil_chega_a_todos_os_aparelhos_conectados() {
    let s = servidor().await;
    let (_, token_a) = parear(&s, "iPhone");
    let (_, token_b) = parear(&s, "iPad");
    let mut a = conectar(&s, &token_a).await;
    let mut b = conectar(&s, &token_b).await;

    s.perfil.send(1).unwrap();

    assert_eq!(proxima(&mut a).await, vec![4, 1]);
    assert_eq!(proxima(&mut b).await, vec![4, 1]);
}

#[tokio::test]
async fn reafirmar_o_mesmo_perfil_tambem_notifica() {
    // usado quando a troca falha: o celular precisa saber que continua no perfil antigo
    let s = servidor().await;
    let (_, token) = parear(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;
    s.perfil.send(0).unwrap();
    assert_eq!(proxima(&mut ws).await, vec![4, 0]);
}

// ---------- revogação ----------

#[tokio::test]
async fn aparelho_revogado_nao_conecta_mais() {
    let s = servidor().await;
    let (id, token) = parear(&s, "iPhone");
    s.acesso.dispositivos().revogar(&id).unwrap();
    let mut ws = connect_async(url_com(&s, &token)).await.unwrap().0;
    assert_eq!(proxima(&mut ws).await, vec![5, 1]);
}

#[tokio::test]
async fn revogar_derruba_a_conexao_que_ja_estava_aberta_sem_afetar_os_outros() {
    let mut s = servidor().await;
    let (id_a, token_a) = parear(&s, "iPhone");
    let (_, token_b) = parear(&s, "iPad");
    let mut a = conectar(&s, &token_a).await;
    let mut b = conectar(&s, &token_b).await;

    s.acesso.dispositivos().revogar(&id_a).unwrap();

    // o próximo sinal do aparelho revogado já é recusado e a conexão cai
    a.send(ping()).await.unwrap();
    assert_eq!(proxima(&mut a).await, vec![5, 1]);
    assert!(foi_encerrada(&mut a).await);

    // o outro segue normalmente
    while let Ok(Some(e)) = tokio::time::timeout(Duration::from_millis(300), s.eventos.recv()).await {
        let _ = e; // descarta o SoltarTudo da conexão derrubada
    }
    b.send(botao(1, true)).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::Botao { id: 1, pressionado: true });
}

// ---------- inatividade ----------

#[tokio::test]
async fn aparelho_em_silencio_e_desconectado_e_solta_os_botoes() {
    let mut s = servidor_com(Duration::from_millis(300)).await;
    let (_, token) = parear(&s, "iPhone");
    let mut mudo = conectar(&s, &token).await;

    assert!(foi_encerrada(&mut mudo).await);
    assert_eq!(proximo(&mut s.eventos).await, Evento::SoltarTudo);
}

#[tokio::test]
async fn aparelho_ativo_nao_e_desconectado() {
    let s = servidor_com(Duration::from_millis(400)).await;
    let (_, token) = parear(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;

    for _ in 0..12 {
        ws.send(ping()).await.unwrap();
        assert_eq!(proxima(&mut ws).await.first(), Some(&3));
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

// ---------- modo teclado: permissão por aparelho e confirmação no PC ----------

const TECLADO: u8 = 3; // índice do perfil "teclado" em PERFIS

fn pedir_perfil(indice: u8) -> Message {
    Message::Binary(vec![4, indice].into())
}

#[tokio::test]
async fn sem_permissao_o_pedido_de_teclado_e_recusado_e_nao_chega_ao_controle() {
    let mut s = servidor().await;
    let (_, token) = parear(&s, "iPad");
    let mut ws = conectar(&s, &token).await;

    ws.send(pedir_perfil(TECLADO)).await.unwrap();
    assert_eq!(proxima(&mut ws).await, vec![6, 1], "sem permissão");
    silencio(&mut s.eventos).await;
    assert!(s.acesso.aprovacoes().pendente().is_none(), "nem chega a perguntar ao dono do PC");
}

#[tokio::test]
async fn com_permissao_o_pc_precisa_confirmar_e_so_entao_o_perfil_muda() {
    let mut s = servidor().await;
    let (_, token) = parear_teclado(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;

    ws.send(pedir_perfil(TECLADO)).await.unwrap();
    assert_eq!(proxima(&mut ws).await, vec![6, 2], "aguardando o PC");
    let pedido = pedido_pendente(&s).await;
    assert!(matches!(pedido.tipo, Tipo::AtivarTeclado { ref nome } if nome == "iPhone"));
    silencio(&mut s.eventos).await; // ainda não mudou nada

    s.acesso.aprovacoes().responder(pedido.id, Decisao::Aprovado { teclado: false });
    assert_eq!(proximo(&mut s.eventos).await, Evento::TrocarPerfil(TECLADO));
}

#[tokio::test]
async fn negar_no_pc_avisa_o_celular_e_nada_muda() {
    let mut s = servidor().await;
    let (_, token) = parear_teclado(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;
    ws.send(pedir_perfil(TECLADO)).await.unwrap();
    assert_eq!(proxima(&mut ws).await, vec![6, 2]);

    let pedido = pedido_pendente(&s).await;
    s.acesso.aprovacoes().responder(pedido.id, Decisao::Negado);
    assert_eq!(proxima(&mut ws).await, vec![6, 3]);
    silencio(&mut s.eventos).await;
}

#[tokio::test]
async fn sem_resposta_no_pc_o_celular_e_avisado_de_que_expirou() {
    let mut s = servidor_completo(Duration::from_secs(6), Duration::from_millis(150)).await;
    let (_, token) = parear_teclado(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;
    ws.send(pedir_perfil(TECLADO)).await.unwrap();
    assert_eq!(proxima(&mut ws).await, vec![6, 2]);
    assert_eq!(proxima(&mut ws).await, vec![6, 4]);
    silencio(&mut s.eventos).await;
}

#[tokio::test]
async fn o_celular_que_cai_enquanto_espera_cancela_o_pedido() {
    let s = servidor().await;
    let (_, token) = parear_teclado(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;
    ws.send(pedir_perfil(TECLADO)).await.unwrap();
    assert_eq!(proxima(&mut ws).await, vec![6, 2]);
    pedido_pendente(&s).await;

    drop(ws);
    for _ in 0..200 {
        if s.acesso.aprovacoes().pendente().is_none() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("o pedido continuou pendente depois que o celular desconectou");
}

#[tokio::test]
async fn pedir_de_novo_enquanto_espera_nao_cria_outro_pedido() {
    let mut s = servidor().await;
    let (_, token) = parear_teclado(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;
    ws.send(pedir_perfil(TECLADO)).await.unwrap();
    ws.send(pedir_perfil(TECLADO)).await.unwrap();
    assert_eq!(proxima(&mut ws).await, vec![6, 2]);
    assert_eq!(proxima(&mut ws).await, vec![6, 2]);

    let pedido = pedido_pendente(&s).await;
    s.acesso.aprovacoes().responder(pedido.id, Decisao::Aprovado { teclado: false });
    assert_eq!(proximo(&mut s.eventos).await, Evento::TrocarPerfil(TECLADO));
    silencio(&mut s.eventos).await; // um único pedido, uma única troca
}

#[tokio::test]
async fn com_o_pc_ocupado_com_outro_pedido_o_celular_e_avisado() {
    let s = servidor().await;
    let (_, token) = parear_teclado(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;

    let ocupando = tokio::spawn({
        let aprovacoes = s.acesso.aprovacoes().clone();
        async move {
            let pedido = Pedido { id: 0, tipo: Tipo::Pareamento { nome: "outro".into() }, origem: None };
            aprovacoes.solicitar(pedido, Duration::from_secs(10)).await
        }
    });
    let outro = pedido_pendente(&s).await;

    ws.send(pedir_perfil(TECLADO)).await.unwrap();
    assert_eq!(proxima(&mut ws).await, vec![6, 5]);

    s.acesso.aprovacoes().responder(outro.id, Decisao::Negado);
    let _ = ocupando.await;
}

#[tokio::test]
async fn se_o_teclado_ja_esta_ativo_o_pedido_passa_direto() {
    let mut s = servidor().await;
    let (_, token) = parear_teclado(&s, "iPhone");
    s.perfil.send(TECLADO).unwrap();
    let mut ws = connect_async(url_com(&s, &token)).await.unwrap().0;
    assert_eq!(proxima(&mut ws).await, vec![4, TECLADO]);

    ws.send(pedir_perfil(TECLADO)).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::TrocarPerfil(TECLADO));
    assert!(s.acesso.aprovacoes().pendente().is_none());
}

#[tokio::test]
async fn aparelho_sem_permissao_continua_trocando_entre_os_perfis_de_controle() {
    let mut s = servidor().await;
    let (_, token) = parear(&s, "iPad");
    let mut ws = conectar(&s, &token).await;
    ws.send(pedir_perfil(1)).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::TrocarPerfil(1));
}

#[tokio::test]
async fn com_o_teclado_ativo_so_aparelhos_com_permissao_conseguem_digitar() {
    let mut s = servidor().await;
    let (_, com) = parear_teclado(&s, "iPhone");
    let (_, sem) = parear(&s, "iPad");
    s.perfil.send(TECLADO).unwrap();
    let mut a = connect_async(url_com(&s, &com)).await.unwrap().0;
    let mut b = connect_async(url_com(&s, &sem)).await.unwrap().0;
    proxima(&mut a).await;
    proxima(&mut b).await;

    b.send(botao(0, true)).await.unwrap();
    silencio(&mut s.eventos).await;
    // o aparelho sem permissão segue conectado e o ping dele continua sendo respondido
    b.send(ping()).await.unwrap();
    assert_eq!(proxima(&mut b).await.first(), Some(&3));

    a.send(botao(1, true)).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::Botao { id: 1, pressionado: true });
}

#[tokio::test]
async fn aparelho_sem_permissao_pode_sair_do_teclado_mas_os_botoes_dele_nao_passam() {
    let mut s = servidor().await;
    let (_, sem) = parear(&s, "iPad");
    s.perfil.send(TECLADO).unwrap();
    let mut b = connect_async(url_com(&s, &sem)).await.unwrap().0;
    proxima(&mut b).await;

    b.send(pedir_perfil(0)).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::TrocarPerfil(0));
}

#[tokio::test]
async fn tirar_a_permissao_com_o_aparelho_conectado_para_as_teclas_na_hora() {
    let mut s = servidor().await;
    let (id, token) = parear_teclado(&s, "iPhone");
    s.perfil.send(TECLADO).unwrap();
    let mut ws = connect_async(url_com(&s, &token)).await.unwrap().0;
    proxima(&mut ws).await;

    ws.send(botao(0, true)).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::Botao { id: 0, pressionado: true });

    s.acesso.dispositivos().definir_teclado(&id, false).unwrap();
    ws.send(botao(0, false)).await.unwrap();
    silencio(&mut s.eventos).await;
}

#[tokio::test]
async fn fora_do_modo_teclado_a_permissao_nao_muda_nada_para_o_gamepad() {
    let mut s = servidor().await;
    let (_, sem) = parear(&s, "iPad");
    let mut ws = conectar(&s, &sem).await;
    ws.send(botao(0, true)).await.unwrap();
    assert_eq!(proximo(&mut s.eventos).await, Evento::Botao { id: 0, pressionado: true });
}

#[tokio::test]
async fn mensagem_maior_que_o_teto_encerra_a_conexao() {
    let mut s = servidor().await;
    let (_, token) = parear(&s, "iPhone");
    let mut ws = conectar(&s, &token).await;

    // o protocolo usa no máximo 9 bytes; 65 é lixo e não pode ser aceito
    let _ = ws.send(Message::Binary(vec![1u8; 65].into())).await;

    assert!(foi_encerrada(&mut ws).await);
    assert_eq!(proximo(&mut s.eventos).await, Evento::SoltarTudo);
}
