use std::{fs, path::PathBuf, sync::Arc, time::{Duration, SystemTime, UNIX_EPOCH}};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tokio::sync::{mpsc, watch};
use tower::ServiceExt;
use vpad::{
    acesso::Acesso, alerta::Alerta, aprovacao::Decisao, armazem::ArmazemMapa, dispositivos::Dispositivos,
    mapa::Mapa, pareamento::Pareamento,
};

struct Ambiente {
    app: Router,
    acesso: Arc<Acesso>,
    armazem: Arc<ArmazemMapa>,
    /// token de um aparelho já pareado
    token: String,
    alertas: Arc<std::sync::Mutex<Vec<Alerta>>>,
}

fn agora() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn ambiente_com(armazem: Arc<ArmazemMapa>) -> Ambiente {
    ambiente_completo(armazem, Duration::from_secs(30))
}

fn ambiente_completo(armazem: Arc<ArmazemMapa>, prazo_da_aprovacao: Duration) -> Ambiente {
    let alertas = Arc::new(std::sync::Mutex::new(vec![]));
    let destino = alertas.clone();
    let acesso = Acesso::com_opcoes(
        Dispositivos::em_memoria(),
        Pareamento::com_segredo(b"segredo-de-teste-123".to_vec()),
        Duration::from_secs(6),
        Arc::new(move |a| destino.lock().unwrap().push(a)),
        prazo_da_aprovacao,
    );
    // o pareamento nasce fechado; os testes que pareiam o abrem
    let token = acesso.dispositivos().parear("teste", 0).unwrap().token;
    let app = vpad::app_com(mpsc::unbounded_channel().0, watch::channel(0).1, armazem.clone(), acesso.clone());
    Ambiente { app, acesso, armazem, token, alertas }
}

fn ambiente() -> Ambiente {
    ambiente_com(Arc::new(ArmazemMapa::em_memoria()))
}

async fn chamar_com(
    app: &Router,
    metodo: &str,
    uri: &str,
    corpo: Option<&str>,
    autorizacao: Option<&str>,
) -> (StatusCode, header::HeaderMap, Value) {
    let mut req = Request::builder().method(metodo).uri(uri);
    if let Some(a) = autorizacao {
        req = req.header(header::AUTHORIZATION, a);
    }
    let body = match corpo {
        Some(c) => {
            req = req.header(header::CONTENT_TYPE, "application/json");
            Body::from(c.to_string())
        }
        None => Body::empty(),
    };
    let res = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, headers, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

async fn teclado(a: &Ambiente, metodo: &str, corpo: Option<&str>) -> (StatusCode, Value) {
    let (s, _, v) = chamar_com(&a.app, metodo, "/api/teclado", corpo, Some(&format!("Bearer {}", a.token))).await;
    (s, v)
}

fn pedido(mapa: Value) -> String {
    json!({ "mapa": mapa }).to_string()
}

// ---------- mapa de teclas (precisa de token de aparelho pareado) ----------

#[tokio::test]
async fn get_devolve_mapa_padrao_controles_e_catalogo_de_teclas() {
    let a = ambiente();
    let (status, corpo) = teclado(&a, "GET", None).await;

    assert_eq!(status, StatusCode::OK);
    let padrao = serde_json::to_value(Mapa::padrao().para_json()).unwrap();
    assert_eq!(corpo["mapa"], padrao);
    assert_eq!(corpo["padrao"], padrao);

    let controles: Vec<_> = corpo["controles"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    assert_eq!(controles.len(), 24);
    assert_eq!(controles[0], "A");
    assert_eq!(controles[23], "RS_Right");

    let teclas = corpo["teclas"].as_array().unwrap();
    assert_eq!(teclas.len(), vpad::teclas::TECLAS.len());
    assert!(teclas.contains(&json!({"id": "KeyA", "rotulo": "A", "grupo": "Letras"})));
}

#[tokio::test]
async fn put_valido_salva_responde_e_aparece_no_get() {
    let a = ambiente();
    let assinante = a.armazem.assinar();

    let (status, corpo) = teclado(&a, "PUT", Some(&pedido(json!({"A": "Space", "RS_Up": "KeyO"})))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(corpo["mapa"], json!({"A": "Space", "RS_Up": "KeyO"}));

    assert!(assinante.has_changed().unwrap(), "o serviço precisa ser avisado");
    let (_, depois) = teclado(&a, "GET", None).await;
    assert_eq!(depois["mapa"], json!({"A": "Space", "RS_Up": "KeyO"}));
    assert_ne!(depois["padrao"], depois["mapa"], "o padrão não muda");
}

#[tokio::test]
async fn put_com_mapa_vazio_desmapeia_tudo() {
    let a = ambiente();
    let (status, corpo) = teclado(&a, "PUT", Some(&pedido(json!({})))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(corpo["mapa"], json!({}));
}

#[tokio::test]
async fn put_com_tecla_ou_controle_desconhecido_e_400_e_nao_muda_nada() {
    let a = ambiente();

    let (status, corpo) = teclado(&a, "PUT", Some(&pedido(json!({"A": "Hyper"})))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(corpo["erro"].as_str().unwrap().contains("Hyper"));

    let (status, corpo) = teclado(&a, "PUT", Some(&pedido(json!({"Turbo": "KeyA"})))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(corpo["erro"].as_str().unwrap().contains("Turbo"));

    assert_eq!(*a.armazem.atual(), Mapa::padrao());
}

#[tokio::test]
async fn put_com_corpo_malformado_e_rejeitado() {
    let a = ambiente();
    for corpo in ["isto nao e json", r#"{"mapa": 3}"#, r#"{"outro": {}}"#] {
        let (status, _) = teclado(&a, "PUT", Some(corpo)).await;
        assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY, "{corpo}: {status}");
    }
}

#[tokio::test]
async fn falha_ao_gravar_em_disco_e_500_e_o_mapa_continua() {
    let pasta: PathBuf = std::env::temp_dir().join(format!("vpad-api-{}", std::process::id()));
    let _ = fs::remove_dir_all(&pasta);
    fs::create_dir_all(&pasta).unwrap();
    fs::write(pasta.join("vpad"), "x").unwrap(); // "vpad" é arquivo: não dá para criar nada dentro
    let a = ambiente_com(Arc::new(ArmazemMapa::com_arquivo(pasta.join("vpad").join("teclado.json"))));

    let (status, corpo) = teclado(&a, "PUT", Some(&pedido(json!({"A": "Space"})))).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(corpo["erro"].is_string());
    assert_eq!(*a.armazem.atual(), Mapa::padrao());
    let _ = fs::remove_dir_all(&pasta);
}

#[tokio::test]
async fn outros_metodos_nao_sao_aceitos() {
    let a = ambiente();
    let (status, _) = teclado(&a, "DELETE", None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn sem_um_token_de_aparelho_pareado_a_api_responde_401_e_nao_muda_nada() {
    let a = ambiente();
    let token = a.token.clone();
    for autorizacao in [
        None,
        Some("Bearer errado".to_string()),
        Some("Bearer ".to_string()),
        Some(token.clone()),                     // sem o prefixo Bearer
        Some(format!("Basic {token}")),
        Some(format!("Bearer {}", token.to_uppercase())),
    ] {
        let (status, _, corpo) = chamar_com(&a.app, "GET", "/api/teclado", None, autorizacao.as_deref()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "GET com {autorizacao:?}");
        assert!(corpo["erro"].is_string());

        let p = pedido(json!({"A": "Space"}));
        let (status, _, _) = chamar_com(&a.app, "PUT", "/api/teclado", Some(&p), autorizacao.as_deref()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "PUT com {autorizacao:?}");
    }
    assert_eq!(*a.armazem.atual(), Mapa::padrao());
}

#[tokio::test]
async fn aparelho_revogado_perde_o_acesso_a_api() {
    let a = ambiente();
    let id = a.acesso.dispositivos().listar()[0].id.clone();
    a.acesso.dispositivos().revogar(&id).unwrap();
    let (status, _) = teclado(&a, "GET", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ---------- pareamento ----------

/// Abre o pareamento no PC e envia o código: sem ninguém aprovando no terminal.
async fn parear(a: &Ambiente, codigo: &str, nome: Option<&str>) -> (StatusCode, header::HeaderMap, Value) {
    a.acesso.pareamento().abrir(agora(), 3600); // o PC abriu o pareamento
    parear_sem_abrir(a, codigo, nome).await
}

async fn parear_sem_abrir(a: &Ambiente, codigo: &str, nome: Option<&str>) -> (StatusCode, header::HeaderMap, Value) {
    let corpo = match nome {
        Some(n) => json!({"codigo": codigo, "nome": n}),
        None => json!({"codigo": codigo}),
    };
    chamar_com(&a.app, "POST", "/api/parear", Some(&corpo.to_string()), None).await
}

/// O PC abre o pareamento, o celular envia o código e quem está no terminal responde `decisao`.
async fn parear_e_responder(
    a: &Ambiente,
    codigo: &str,
    nome: Option<&str>,
    decisao: Decisao,
) -> (StatusCode, header::HeaderMap, Value) {
    a.acesso.pareamento().abrir(agora(), 3600);
    let pedido = parear_sem_abrir(a, codigo, nome);
    let terminal = async {
        for _ in 0..400 {
            if let Some(p) = a.acesso.aprovacoes().pendente() {
                a.acesso.aprovacoes().responder(p.id, decisao);
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("o pedido de aprovação não chegou ao terminal");
    };
    tokio::join!(pedido, terminal).0
}

async fn parear_aprovado(a: &Ambiente, codigo: &str, nome: Option<&str>) -> (StatusCode, header::HeaderMap, Value) {
    parear_e_responder(a, codigo, nome, Decisao::Aprovado { teclado: false }).await
}

fn codigo_certo(a: &Ambiente) -> String {
    a.acesso.pareamento().codigo_atual(agora())
}

fn codigo_errado(a: &Ambiente) -> String {
    // diferente dos dois códigos que valem agora (o atual e o anterior)
    let validos = [agora(), agora().saturating_sub(30)].map(|t| a.acesso.pareamento().codigo_atual(t));
    (0..1000).map(|n| format!("{n:06}")).find(|c| !validos.contains(c)).unwrap()
}

#[tokio::test]
async fn o_codigo_certo_aprovado_no_pc_devolve_um_token_exclusivo_que_abre_a_api() {
    let a = ambiente();
    let (status, _, corpo) = parear_aprovado(&a, &codigo_certo(&a), Some("iPhone do Wilian")).await;
    assert_eq!(status, StatusCode::OK);

    let token = corpo["token"].as_str().unwrap().to_string();
    assert_eq!(token.len(), 32);
    assert_eq!(corpo["id"].as_str().unwrap().len(), 8);
    assert!(a.acesso.autorizado(&token));
    assert_ne!(token, a.token, "cada aparelho tem o seu");

    let (status, _, _) = chamar_com(&a.app, "GET", "/api/teclado", None, Some(&format!("Bearer {token}"))).await;
    assert_eq!(status, StatusCode::OK);

    let nomes: Vec<_> = a.acesso.dispositivos().listar().into_iter().map(|d| d.nome).collect();
    assert!(nomes.contains(&"iPhone do Wilian".to_string()));
}

#[tokio::test]
async fn o_celular_fica_esperando_a_resposta_do_pc() {
    let a = ambiente();
    a.acesso.pareamento().abrir(agora(), 3600);
    let codigo = codigo_certo(&a);

    let requisicao = tokio::spawn({
        let app = a.app.clone();
        async move {
            let corpo = json!({"codigo": codigo, "nome": "iPhone"}).to_string();
            chamar_com(&app, "POST", "/api/parear", Some(&corpo), None).await
        }
    });
    // enquanto ninguém responde no terminal, a requisição não termina
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(!requisicao.is_finished());
    assert!(a.acesso.aprovacoes().pendente().is_some());
    assert_eq!(a.acesso.dispositivos().listar().len(), 1, "só o aparelho de teste: nada foi cadastrado");

    let id = a.acesso.aprovacoes().pendente().unwrap().id;
    a.acesso.aprovacoes().responder(id, Decisao::Aprovado { teclado: false });
    assert_eq!(requisicao.await.unwrap().0, StatusCode::OK);
}

#[tokio::test]
async fn aprovar_com_teclado_libera_o_modo_teclado_do_novo_aparelho() {
    let a = ambiente();
    let (_, _, corpo) = parear_e_responder(&a, &codigo_certo(&a), None, Decisao::Aprovado { teclado: true }).await;
    assert!(a.acesso.dispositivos().teclado_permitido(corpo["token"].as_str().unwrap()));
}

#[tokio::test]
async fn o_nome_e_opcional() {
    let a = ambiente();
    let (status, _, _) = parear_aprovado(&a, &codigo_certo(&a), None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn negado_no_pc_e_403_sem_token() {
    let a = ambiente();
    let antes = a.acesso.dispositivos().listar().len();
    let (status, _, corpo) = parear_e_responder(&a, &codigo_certo(&a), Some("intruso"), Decisao::Negado).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(corpo["erro"].as_str().unwrap().to_lowercase().contains("negado"));
    assert!(corpo.get("token").is_none());
    assert_eq!(a.acesso.dispositivos().listar().len(), antes);
}

#[tokio::test]
async fn ninguem_aprovar_no_prazo_e_408() {
    let a = ambiente_completo(Arc::new(ArmazemMapa::em_memoria()), Duration::from_millis(100));
    let (status, _, corpo) = parear(&a, &codigo_certo(&a), None).await;
    assert_eq!(status, StatusCode::REQUEST_TIMEOUT);
    assert!(corpo["erro"].as_str().unwrap().to_lowercase().contains("aprov"));
}

#[tokio::test]
async fn depois_de_um_pareamento_a_janela_fecha_e_um_novo_precisa_ser_aberto_no_pc() {
    let a = ambiente();
    assert_eq!(parear_aprovado(&a, &codigo_certo(&a), None).await.0, StatusCode::OK);
    // outro código válido, mas ninguém reabriu o pareamento no terminal
    let outro = a.acesso.pareamento().codigo_atual(agora() + 30);
    let (status, _, corpo) = parear_sem_abrir(&a, &outro, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(corpo["erro"].as_str().unwrap().to_lowercase().contains("fechado"));
}

#[tokio::test]
async fn pedido_enquanto_outro_espera_aprovacao_no_pc_e_409() {
    use vpad::aprovacao::{Pedido, Tipo};
    let a = ambiente();
    a.acesso.pareamento().abrir(agora(), 3600);

    // outro pedido (um aparelho querendo ativar o modo teclado) já espera no terminal
    let outro = tokio::spawn({
        let aprovacoes = a.acesso.aprovacoes().clone();
        async move {
            let pedido = Pedido { id: 0, tipo: Tipo::AtivarTeclado { nome: "iPad".into() }, origem: None };
            aprovacoes.solicitar(pedido, Duration::from_secs(10)).await
        }
    });
    for _ in 0..200 {
        if a.acesso.aprovacoes().pendente().is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    let (status, _, corpo) = parear_sem_abrir(&a, &codigo_certo(&a), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(corpo["erro"].as_str().unwrap().to_lowercase().contains("aguardando"));
    assert!(a.acesso.pareamento().aberto(agora()), "o código foi gasto, mas a janela continua aberta");

    let id = a.acesso.aprovacoes().pendente().unwrap().id;
    a.acesso.aprovacoes().responder(id, Decisao::Negado);
    let _ = outro.await;
}

#[tokio::test]
async fn codigo_errado_e_401_e_nao_pareia_nem_pede_aprovacao() {
    let a = ambiente();
    let antes = a.acesso.dispositivos().listar().len();
    let (status, _, corpo) = parear(&a, &codigo_errado(&a), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(corpo["erro"].as_str().unwrap().to_lowercase().contains("código"));
    assert_eq!(a.acesso.dispositivos().listar().len(), antes);
    assert!(a.acesso.aprovacoes().pendente().is_none());
}

#[tokio::test]
async fn cada_codigo_so_pareia_uma_vez() {
    let a = ambiente();
    let codigo = codigo_certo(&a);
    assert_eq!(parear_aprovado(&a, &codigo, None).await.0, StatusCode::OK);
    assert_eq!(parear(&a, &codigo, None).await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn muitas_tentativas_erradas_bloqueiam_ate_o_codigo_certo_com_429() {
    let a = ambiente();
    let errado = codigo_errado(&a);
    for _ in 0..5 {
        assert_eq!(parear(&a, &errado, None).await.0, StatusCode::UNAUTHORIZED);
    }
    let (status, headers, corpo) = parear(&a, &codigo_certo(&a), None).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let espera: u64 = headers[header::RETRY_AFTER].to_str().unwrap().parse().unwrap();
    assert!((1..=60).contains(&espera), "{espera}");
    assert_eq!(corpo["segundos"].as_u64().unwrap(), espera);
}

#[tokio::test]
async fn corpo_malformado_no_pareamento_e_rejeitado() {
    let a = ambiente();
    for corpo in ["nao e json", r#"{"outro": 1}"#, r#"{"codigo": 123456}"#] {
        let (status, _, _) = chamar_com(&a.app, "POST", "/api/parear", Some(corpo), None).await;
        assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY, "{corpo}: {status}");
    }
}

#[tokio::test]
async fn pareamento_so_aceita_post() {
    let a = ambiente();
    let (status, _, _) = chamar_com(&a.app, "GET", "/api/parear", None, None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

// ---------- pareamento fechado e alertas ----------

#[tokio::test]
async fn com_o_pareamento_fechado_o_codigo_certo_e_recusado_com_403_e_instrucao() {
    let a = ambiente();
    let antes = a.acesso.dispositivos().listar().len();
    let (status, _, corpo) = parear_sem_abrir(&a, &codigo_certo(&a), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let erro = corpo["erro"].as_str().unwrap().to_lowercase();
    assert!(erro.contains("fechado") && erro.contains("enter"), "{erro}");
    assert_eq!(a.acesso.dispositivos().listar().len(), antes);
}

#[tokio::test]
async fn abrir_o_pareamento_no_pc_libera_o_mesmo_codigo() {
    let a = ambiente();
    let codigo = codigo_certo(&a);
    assert_eq!(parear_sem_abrir(&a, &codigo, None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(parear_aprovado(&a, &codigo, None).await.0, StatusCode::OK);
}

#[tokio::test]
async fn os_alertas_levam_o_ip_de_quem_tentou() {
    let a = ambiente();
    a.acesso.pareamento().abrir(agora(), 3600);
    let origem: std::net::SocketAddr = "192.168.68.77:51234".parse().unwrap();
    let corpo = json!({"codigo": codigo_errado(&a)}).to_string();
    let mut req = Request::builder()
        .method("POST")
        .uri("/api/parear")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(corpo))
        .unwrap();
    req.extensions_mut().insert(axum::extract::ConnectInfo(origem));
    let res = a.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    assert_eq!(
        *a.alertas.lock().unwrap(),
        vec![Alerta::CodigoErrado { origem: Some(origem.ip()), erros: 1 }]
    );
}

#[tokio::test]
async fn sem_informacao_de_origem_o_alerta_diz_que_e_desconhecida() {
    let a = ambiente();
    parear(&a, &codigo_errado(&a), None).await;
    assert_eq!(*a.alertas.lock().unwrap(), vec![Alerta::CodigoErrado { origem: None, erros: 1 }]);
}

#[tokio::test]
async fn tentativa_com_o_pareamento_fechado_tambem_gera_alerta() {
    let a = ambiente();
    parear_sem_abrir(&a, &codigo_certo(&a), None).await;
    assert_eq!(*a.alertas.lock().unwrap(), vec![Alerta::PareamentoFechado { origem: None }]);
}
