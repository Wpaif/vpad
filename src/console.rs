//! O terminal do PC como painel de controle de segurança: quem está diante dele abre o
//! pareamento (Enter), abre a página do certificado (`c`) e responde aos pedidos de aprovação
//! (`s`, `t` ou negar). É uma máquina de estados pura: recebe linhas digitadas e "tiques" de
//! um segundo e devolve o que deve ser mostrado; quem imprime é o `main`.

use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    acesso::Acesso,
    aprovacao::{Decisao, Tipo},
    cli::linha_do_codigo,
    mensagens::txt,
    tr,
    configuracao::ServidorDeConfiguracao,
};

/// Quanto tempo o pareamento fica aberto depois do Enter.
pub const DURACAO_DO_PAREAMENTO: u64 = 120;
/// Quanto tempo a página de configuração do certificado fica no ar.
pub const DURACAO_DA_CONFIGURACAO: Duration = Duration::from_secs(300);

/// A página de configuração do certificado, vista pelo console (um falso nos testes).
pub trait Configurador: Send + Sync {
    fn abrir(&self, duracao: Duration) -> io::Result<SocketAddr>;
    fn aberto(&self) -> bool;
    fn segundos_restantes(&self) -> u64;
    fn fechar(&self);
}

impl Configurador for Arc<ServidorDeConfiguracao> {
    fn abrir(&self, duracao: Duration) -> io::Result<SocketAddr> {
        ServidorDeConfiguracao::abrir(self, duracao)
    }
    fn aberto(&self) -> bool {
        ServidorDeConfiguracao::aberto(self)
    }
    fn segundos_restantes(&self) -> u64 {
        ServidorDeConfiguracao::segundos_restantes(self)
    }
    fn fechar(&self) {
        ServidorDeConfiguracao::fechar(self)
    }
}

/// O que mostrar no terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Saida {
    /// uma linha permanente (o `main` a imprime e passa para a seguinte)
    Linha(String),
    /// texto que se reescreve por cima da linha atual (o código girando)
    Atualiza(String),
}

#[derive(Default)]
struct Estado {
    /// o último pedido de aprovação já mostrado ao usuário
    pedido_anunciado: Option<u64>,
    pareamento_estava_aberto: bool,
    dica_inicial_mostrada: bool,
    configuracao_estava_aberta: bool,
}

pub struct Console {
    acesso: Arc<Acesso>,
    configurador: Arc<dyn Configurador>,
    url_de_configuracao: String,
    estado: Mutex<Estado>,
}

/// "Sim e liberar o modo teclado": `t` (de teclado) ou `k` (de keyboard), para quem usa o terminal
/// em português ou em inglês.
fn liberar_teclado(resposta: &str) -> bool {
    matches!(resposta.trim().to_lowercase().as_str(), "t" | "teclado" | "k" | "keyboard")
}

fn afirmativa(resposta: &str) -> bool {
    matches!(resposta.trim().to_lowercase().as_str(), "s" | "sim" | "y" | "yes")
}

impl Console {
    pub fn novo(acesso: Arc<Acesso>, configurador: Arc<dyn Configurador>, url_de_configuracao: String) -> Self {
        Self { acesso, configurador, url_de_configuracao, estado: Mutex::new(Estado::default()) }
    }

    fn ajuda() -> Vec<Saida> {
        let (abrir, certificado) = (DURACAO_DO_PAREAMENTO / 60, DURACAO_DA_CONFIGURACAO.as_secs() / 60);
        [
            tr!("comandos (digite e tecle Enter):", "commands (type and press Enter):"),
            tr!(
                "  Enter  abrir o pareamento por {abrir} minutos (mostra o código para digitar no celular)",
                "  Enter  open pairing for {abrir} minutes (shows the code to type on the phone)"
            ),
            tr!("  f      fechar o pareamento agora", "  f      close pairing now"),
            tr!(
                "  c      abrir por {certificado} minutos a página que instala o certificado no celular",
                "  c      open the certificate setup page for {certificado} minutes (installs the certificate on the phone)"
            ),
            tr!("  ?      esta ajuda", "  ?      this help"),
            tr!(
                "quando um celular pedir aprovação: s = sim, t = sim e liberar o modo teclado, outra tecla = negar",
                "when a phone asks for approval: y = yes, k = yes and allow keyboard mode, any other key = deny"
            ),
        ]
        .into_iter()
        .map(Saida::Linha)
        .collect()
    }

    /// Uma linha digitada pelo usuário (sem o `\n`). `agora` são segundos desde 1970.
    pub fn linha(&self, texto: &str, agora: u64) -> Vec<Saida> {
        // Com um pedido esperando, a linha é a resposta a ele: negar é o padrão.
        if let Some(pedido) = self.acesso.aprovacoes().pendente() {
            let teclado = matches!(pedido.tipo, Tipo::Pareamento { .. })
                && liberar_teclado(texto);
            let (decisao, mensagem) = if teclado {
                (Decisao::Aprovado { teclado: true }, txt("✓ aprovado, com o modo teclado liberado para o aparelho", "✓ approved, keyboard mode allowed for the device"))
            } else if afirmativa(texto) || matches!(pedido.tipo, Tipo::AtivarTeclado { .. }) && liberar_teclado(texto) {
                (Decisao::Aprovado { teclado: false }, txt("✓ aprovado", "✓ approved"))
            } else {
                (Decisao::Negado, txt("pedido negado", "request denied"))
            };
            let aplicado = self.acesso.aprovacoes().responder(pedido.id, decisao);
            let mensagem = if aplicado { mensagem } else { txt("o pedido já foi encerrado", "the request has already ended") };
            return vec![Saida::Linha(mensagem.to_string())];
        }

        match texto.trim().to_lowercase().as_str() {
            "" => {
                let ja_estava_aberto = self.acesso.pareamento().aberto(agora);
                self.acesso.pareamento().abrir(agora, DURACAO_DO_PAREAMENTO);
                let aviso = if ja_estava_aberto {
                    tr!("pareamento renovado por mais {DURACAO_DO_PAREAMENTO} s", "pairing renewed for {DURACAO_DO_PAREAMENTO} s more")
                } else {
                    tr!(
                        "pareamento ABERTO por {DURACAO_DO_PAREAMENTO} s: digite no celular o código abaixo",
                        "pairing OPEN for {DURACAO_DO_PAREAMENTO} s: type the code below on the phone"
                    )
                };
                vec![Saida::Linha(aviso)]
            }
            "f" | "fechar" | "close" => {
                self.acesso.pareamento().fechar();
                vec![Saida::Linha(txt("pareamento fechado", "pairing closed").to_string())]
            }
            "c" => match self.configurador.abrir(DURACAO_DA_CONFIGURACAO) {
                Ok(_) => vec![Saida::Linha(tr!(
                    "página de configuração do certificado aberta por {} s: abra no celular {}",
                    "certificate setup page open for {} s: open on the phone {}",
                    DURACAO_DA_CONFIGURACAO.as_secs(),
                    self.url_de_configuracao
                ))],
                Err(e) => vec![Saida::Linha(tr!(
                    "não consegui abrir a página de configuração: {e}",
                    "could not open the setup page: {e}"
                ))],
            },
            _ => Self::ajuda(),
        }
    }

    /// Chamado a cada segundo: devolve o que mudou na tela.
    pub fn tique(&self, agora: u64) -> Vec<Saida> {
        let mut estado = self.estado.lock().unwrap();
        let mut saidas = vec![];

        match self.acesso.aprovacoes().pendente() {
            Some(pedido) => {
                // Com um pedido esperando, a tela é dele: sem o código girando por cima.
                if estado.pedido_anunciado != Some(pedido.id) {
                    estado.pedido_anunciado = Some(pedido.id);
                    let opcoes = match pedido.tipo {
                        Tipo::Pareamento { .. } => txt(
                            "[s] sim   [t] sim e liberar o modo teclado   [Enter ou outra tecla] negar",
                            "[y] yes   [k] yes and allow keyboard mode   [Enter or any other key] deny",
                        ),
                        Tipo::AtivarTeclado { .. } => txt("[s] sim   [Enter ou outra tecla] negar", "[y] yes   [Enter or any other key] deny"),
                    };
                    saidas.push(Saida::Linha(tr!("⚠ pedido: {}\n  {opcoes}", "⚠ request: {}\n  {opcoes}", pedido.descricao())));
                }
                return saidas;
            }
            None => estado.pedido_anunciado = None,
        }

        let p = self.acesso.pareamento();
        if p.aberto(agora) {
            estado.pareamento_estava_aberto = true;
            estado.dica_inicial_mostrada = true; // quem abriu o pareamento já sabe como se faz
            saidas.push(Saida::Atualiza(tr!(
                "{}   [pareamento fecha em {} s]",
                "{}   [pairing closes in {} s]",
                linha_do_codigo(&p.codigo_atual(agora), p.segundos_restantes_do_codigo(agora)),
                p.segundos_abertos_restantes(agora)
            )));
        } else if estado.pareamento_estava_aberto {
            estado.pareamento_estava_aberto = false;
            saidas.push(Saida::Linha(txt(
                "pareamento FECHADO: pressione Enter para abrir de novo",
                "pairing CLOSED: press Enter to open it again",
            ).to_string()));
        } else if !estado.dica_inicial_mostrada {
            estado.dica_inicial_mostrada = true;
            let minutos = DURACAO_DO_PAREAMENTO / 60;
            saidas.push(Saida::Linha(tr!(
                "pareamento FECHADO: pressione Enter para abrir por {minutos} minutos (c = página do certificado, ? = ajuda)",
                "pairing CLOSED: press Enter to open it for {minutos} minutes (c = certificate page, ? = help)"
            )));
        }

        if self.configurador.aberto() {
            estado.configuracao_estava_aberta = true;
        } else if estado.configuracao_estava_aberta {
            estado.configuracao_estava_aberta = false;
            saidas.push(Saida::Linha(txt("página de configuração fechada", "setup page closed").to_string()));
        }
        saidas
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::{
        aprovacao::{Decisao, Pedido, Tipo},
        dispositivos::Dispositivos,
        pareamento::Pareamento,
    };
    use std::sync::Mutex;

    const T0: u64 = 1_700_000_010;

    #[derive(Default)]
    struct Falso {
        aberto: Mutex<bool>,
        pedidos: Mutex<Vec<Duration>>,
        falhar: Mutex<bool>,
    }
    impl Configurador for Falso {
        fn abrir(&self, duracao: Duration) -> io::Result<SocketAddr> {
            if *self.falhar.lock().unwrap() {
                return Err(io::Error::new(io::ErrorKind::AddrInUse, "porta ocupada"));
            }
            self.pedidos.lock().unwrap().push(duracao);
            *self.aberto.lock().unwrap() = true;
            Ok("0.0.0.0:8080".parse().unwrap())
        }
        fn aberto(&self) -> bool {
            *self.aberto.lock().unwrap()
        }
        fn segundos_restantes(&self) -> u64 {
            if self.aberto() { 250 } else { 0 }
        }
        fn fechar(&self) {
            *self.aberto.lock().unwrap() = false;
        }
    }

    fn console() -> (Console, Arc<Acesso>, Arc<Falso>) {
        let acesso = Acesso::novo(
            Dispositivos::em_memoria(),
            Pareamento::com_segredo(b"segredo-de-teste-123".to_vec()),
            Duration::from_secs(6),
        );
        let falso = Arc::new(Falso::default());
        let c = Console::novo(acesso.clone(), falso.clone(), "http://192.168.68.103:8080/".into());
        (c, acesso, falso)
    }

    fn texto(saidas: &[Saida]) -> String {
        saidas
            .iter()
            .map(|s| match s {
                Saida::Linha(t) | Saida::Atualiza(t) => t.clone(),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn linhas(saidas: &[Saida]) -> usize {
        saidas.iter().filter(|s| matches!(s, Saida::Linha(_))).count()
    }

    fn pedido_de_pareamento(nome: &str) -> Pedido {
        Pedido { id: 0, tipo: Tipo::Pareamento { nome: nome.into() }, origem: Some("192.168.68.50".parse().unwrap()) }
    }

    /// Faz o pedido numa tarefa e espera ele aparecer; devolve a tarefa.
    async fn pedir(acesso: &Arc<Acesso>, pedido: Pedido) -> tokio::task::JoinHandle<Result<Decisao, crate::aprovacao::Ocupado>> {
        let a = acesso.aprovacoes().clone();
        let tarefa = tokio::spawn(async move { a.solicitar(pedido, Duration::from_secs(10)).await });
        for _ in 0..200 {
            if acesso.aprovacoes().pendente().is_some() {
                return tarefa;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("o pedido não apareceu");
    }

    // ---------- pareamento ----------

    #[test]
    fn de_inicio_diz_que_o_pareamento_esta_fechado_e_como_abrir_so_uma_vez() {
        let (c, _, _) = console();
        let primeiro = texto(&c.tique(T0));
        assert!(primeiro.contains("FECHADO") && primeiro.contains("Enter"), "{primeiro}");
        assert!(c.tique(T0 + 1).is_empty(), "a dica não se repete a cada segundo");
    }

    #[test]
    fn enter_abre_o_pareamento_por_dois_minutos() {
        let (c, acesso, _) = console();
        let saida = texto(&c.linha("", T0));
        assert!(acesso.pareamento().aberto(T0 + 119));
        assert!(!acesso.pareamento().aberto(T0 + 120));
        assert!(saida.contains("ABERTO") && saida.contains("120"), "{saida}");
    }

    #[test]
    fn com_o_pareamento_aberto_o_codigo_gira_na_mesma_linha() {
        let (c, acesso, _) = console();
        c.linha("", T0);
        let saidas = c.tique(T0 + 5);
        assert_eq!(linhas(&saidas), 0, "não enche a tela de linhas novas");
        let t = texto(&saidas);
        let codigo = acesso.pareamento().codigo_atual(T0 + 5);
        assert!(t.contains(&format!("{} {}", &codigo[..3], &codigo[3..])), "{t}");
        assert!(t.contains("115"), "mostra quanto falta para o pareamento fechar: {t}");
    }

    #[test]
    fn abrir_de_novo_renova_e_avisa() {
        let (c, acesso, _) = console();
        c.linha("", T0);
        let t = texto(&c.linha("", T0 + 100));
        assert!(t.to_lowercase().contains("renov"), "{t}");
        assert!(acesso.pareamento().aberto(T0 + 200));
    }

    #[test]
    fn quando_a_janela_acaba_avisa_uma_vez_e_o_codigo_para_de_aparecer() {
        let (c, _, _) = console();
        c.linha("", T0);
        c.tique(T0 + 1);
        let fim = texto(&c.tique(T0 + 121));
        assert!(fim.contains("FECHADO"), "{fim}");
        assert!(c.tique(T0 + 122).is_empty());
    }

    #[test]
    fn f_fecha_o_pareamento_na_hora() {
        let (c, acesso, _) = console();
        c.linha("", T0);
        let t = texto(&c.linha("f", T0 + 1));
        assert!(!acesso.pareamento().aberto(T0 + 2));
        assert!(t.to_lowercase().contains("fechado"), "{t}");
    }

    // ---------- pedidos de aprovação ----------

    #[tokio::test]
    async fn um_pedido_novo_e_anunciado_uma_vez_com_as_opcoes() {
        let (c, acesso, _) = console();
        let _tarefa = pedir(&acesso, pedido_de_pareamento("iPhone do Wilian")).await;

        let t = texto(&c.tique(T0));
        assert!(t.contains("iPhone do Wilian") && t.contains("192.168.68.50"), "{t}");
        assert!(t.contains("[s]") && t.contains("[t]"), "pareamento oferece liberar o modo teclado: {t}");
        assert!(c.tique(T0 + 1).is_empty(), "não repete o pedido");
    }

    #[tokio::test]
    async fn enquanto_um_pedido_espera_o_codigo_nao_aparece() {
        let (c, acesso, _) = console();
        c.linha("", T0);
        let _tarefa = pedir(&acesso, pedido_de_pareamento("iPhone")).await;
        c.tique(T0 + 1); // anuncia o pedido
        assert!(c.tique(T0 + 2).is_empty());
    }

    #[tokio::test]
    async fn s_aprova_sem_o_modo_teclado() {
        let (c, acesso, _) = console();
        let tarefa = pedir(&acesso, pedido_de_pareamento("iPhone")).await;
        let t = texto(&c.linha("s", T0));
        assert_eq!(tarefa.await.unwrap(), Ok(Decisao::Aprovado { teclado: false }));
        assert!(t.to_lowercase().contains("aprovado"), "{t}");
    }

    #[tokio::test]
    async fn t_aprova_e_libera_o_modo_teclado() {
        let (c, acesso, _) = console();
        let tarefa = pedir(&acesso, pedido_de_pareamento("iPhone")).await;
        let t = texto(&c.linha("t", T0));
        assert_eq!(tarefa.await.unwrap(), Ok(Decisao::Aprovado { teclado: true }));
        assert!(t.to_lowercase().contains("teclado"), "{t}");
    }

    #[tokio::test]
    async fn k_e_keyboard_liberam_o_teclado_como_o_t_para_quem_usa_o_terminal_em_ingles() {
        for resposta in ["t", "teclado", "k", "K", "keyboard", " Keyboard "] {
            let (c, acesso, _) = console();
            let tarefa = pedir(&acesso, pedido_de_pareamento("iPhone")).await;
            c.linha(resposta, T0);
            assert_eq!(tarefa.await.unwrap(), Ok(Decisao::Aprovado { teclado: true }), "{resposta:?}");
        }
    }

    #[tokio::test]
    async fn no_pedido_de_ativar_o_teclado_o_k_aprova_sem_liberar_nada_a_mais() {
        for resposta in ["t", "k", "s", "y"] {
            let (c, acesso, _) = console();
            let pedido = Pedido { id: 0, tipo: Tipo::AtivarTeclado { nome: "iPhone".into() }, origem: None };
            let tarefa = pedir(&acesso, pedido).await;
            c.linha(resposta, T0);
            assert_eq!(tarefa.await.unwrap(), Ok(Decisao::Aprovado { teclado: false }), "{resposta:?}");
        }
    }

    #[test]
    fn k_sem_pedido_nao_faz_nada_alem_de_mostrar_a_ajuda() {
        let (c, acesso, _) = console();
        let t = texto(&c.linha("k", T0));
        assert!(!acesso.pareamento().aberto(T0));
        assert!(t.contains("Enter"), "{t}");
    }

    #[test]
    fn close_e_fechar_fecham_o_pareamento_como_o_f() {
        for comando in ["f", "fechar", "close", " CLOSE "] {
            let (c, acesso, _) = console();
            c.linha("", T0);
            assert!(acesso.pareamento().aberto(T0), "{comando:?}");
            c.linha(comando, T0 + 1);
            assert!(!acesso.pareamento().aberto(T0 + 1), "{comando:?}");
        }
    }

    #[tokio::test]
    async fn qualquer_outra_resposta_nega_e_o_padrao_e_negar() {
        for resposta in ["", "n", "nao", "não", "talvez", "ss", " ", "tudo"] {
            let (c, acesso, _) = console();
            let tarefa = pedir(&acesso, pedido_de_pareamento("iPhone")).await;
            let t = texto(&c.linha(resposta, T0));
            assert_eq!(tarefa.await.unwrap(), Ok(Decisao::Negado), "{resposta:?}");
            assert!(t.to_lowercase().contains("negado"), "{t}");
        }
    }

    #[tokio::test]
    async fn aceita_maiusculas_e_espacos_nas_respostas_afirmativas() {
        for resposta in ["S", " s ", "SIM", "sim", "y", "yes"] {
            let (c, acesso, _) = console();
            let tarefa = pedir(&acesso, pedido_de_pareamento("iPhone")).await;
            c.linha(resposta, T0);
            assert_eq!(tarefa.await.unwrap(), Ok(Decisao::Aprovado { teclado: false }), "{resposta:?}");
        }
    }

    #[tokio::test]
    async fn pedido_de_ativar_o_teclado_nao_oferece_liberar_permanentemente() {
        let (c, acesso, _) = console();
        let pedido = Pedido { id: 0, tipo: Tipo::AtivarTeclado { nome: "iPhone".into() }, origem: None };
        let tarefa = pedir(&acesso, pedido).await;
        let t = texto(&c.tique(T0));
        assert!(t.to_lowercase().contains("modo teclado") && t.contains("[s]"), "{t}");
        assert!(!t.contains("[t]"), "{t}");
        c.linha("s", T0);
        assert!(matches!(tarefa.await.unwrap(), Ok(Decisao::Aprovado { .. })));
    }

    #[test]
    fn s_sem_pedido_nao_faz_nada_alem_de_mostrar_a_ajuda() {
        let (c, acesso, _) = console();
        let t = texto(&c.linha("s", T0));
        assert!(!acesso.pareamento().aberto(T0), "um 's' perdido não abre nem aprova nada");
        assert!(t.to_lowercase().contains("ajuda") || t.contains("Enter"), "{t}");
    }

    #[tokio::test]
    async fn depois_do_pedido_o_codigo_volta_a_girar_se_o_pareamento_continua_aberto() {
        let (c, acesso, _) = console();
        let tarefa = pedir(&acesso, pedido_de_pareamento("iPhone")).await;
        c.tique(T0);
        acesso.pareamento().abrir(T0, 120);
        c.linha("n", T0 + 1);
        tarefa.await.unwrap().unwrap();
        let t = texto(&c.tique(T0 + 2));
        assert!(t.contains("código de pareamento"), "{t}");
    }

    // ---------- página de configuração do certificado ----------

    #[test]
    fn c_abre_a_pagina_de_configuracao_por_cinco_minutos_e_mostra_o_endereco() {
        let (c, _, falso) = console();
        let t = texto(&c.linha("c", T0));
        assert_eq!(*falso.pedidos.lock().unwrap(), vec![Duration::from_secs(300)]);
        assert!(t.contains("http://192.168.68.103:8080/") && t.contains("300"), "{t}");
    }

    #[test]
    fn se_a_porta_estiver_ocupada_o_erro_aparece() {
        let (c, _, falso) = console();
        *falso.falhar.lock().unwrap() = true;
        let t = texto(&c.linha("c", T0));
        assert!(t.contains("porta ocupada") && !falso.aberto(), "{t}");
    }

    #[test]
    fn quando_a_pagina_fecha_o_terminal_avisa_uma_vez() {
        let (c, _, falso) = console();
        c.linha("c", T0);
        c.tique(T0 + 1);
        falso.fechar();
        assert!(texto(&c.tique(T0 + 2)).contains("configuração fechada"));
        assert!(!texto(&c.tique(T0 + 3)).contains("configuração fechada"));
    }

    // ---------- ajuda ----------

    #[test]
    fn a_ajuda_lista_os_comandos() {
        let (c, _, _) = console();
        let t = texto(&c.linha("?", T0));
        for comando in ["Enter", "c", "f"] {
            assert!(t.contains(comando), "{comando}: {t}");
        }
        assert!(!texto(&c.linha("ajuda", T0)).is_empty());
    }

    // ---------- terminal em inglês ----------

    use crate::{idiomas::Idioma, mensagens::com_idioma};

    fn em_ingles<R>(f: impl FnOnce() -> R) -> R {
        com_idioma(Idioma::En, f)
    }

    #[test]
    fn a_ajuda_em_ingles_mostra_as_letras_do_idioma_e_os_minutos_certos() {
        let (c, _, _) = console();
        let t = em_ingles(|| texto(&c.linha("?", T0)));
        assert!(t.contains("commands (type and press Enter)"), "{t}");
        assert!(t.contains("open pairing for 2 minutes"), "{t}");
        assert!(t.contains("open the certificate setup page for 5 minutes"), "{t}");
        assert!(t.contains("y = yes, k = yes and allow keyboard mode"), "{t}");
        assert!(!t.contains("comandos") && !t.contains("sim"), "nada de português: {t}");
    }

    #[test]
    fn a_ajuda_em_portugues_continua_igual() {
        let (c, _, _) = console();
        let t = texto(&c.linha("?", T0));
        assert!(t.contains("comandos (digite e tecle Enter)"), "{t}");
        assert!(t.contains("abrir o pareamento por 2 minutos"), "{t}");
        assert!(t.contains("s = sim, t = sim e liberar o modo teclado"), "{t}");
    }

    #[tokio::test]
    async fn respostas_de_aprovacao_em_ingles() {
        for (resposta, esperado) in [
            ("y", "approved"),
            ("k", "keyboard mode allowed"),
            ("n", "request denied"),
            ("", "request denied"),
        ] {
            let (c, acesso, _) = console();
            let _tarefa = pedir(&acesso, pedido_de_pareamento("iPhone")).await;
            let t = em_ingles(|| texto(&c.linha(resposta, T0)));
            assert!(t.contains(esperado), "{resposta:?}: {t}");
            assert!(!t.contains("aprovado") && !t.contains("negado"), "{resposta:?}: {t}");
        }
    }

    #[test]
    fn abrir_renovar_e_fechar_o_pareamento_em_ingles() {
        let (c, _, _) = console();
        assert!(em_ingles(|| texto(&c.linha("", T0))).contains("pairing OPEN for 120 s: type the code below on the phone"));
        assert!(em_ingles(|| texto(&c.linha("", T0 + 1))).contains("pairing renewed for 120 s more"));
        assert!(em_ingles(|| texto(&c.linha("close", T0 + 2))).contains("pairing closed"));
    }

    #[tokio::test]
    async fn o_anuncio_do_pedido_em_ingles_mostra_as_opcoes_do_idioma() {
        let (c, acesso, _) = console();
        let _tarefa = pedir(&acesso, pedido_de_pareamento("iPhone")).await;
        let t = em_ingles(|| texto(&c.tique(T0)));
        assert!(t.contains("request:"), "{t}");
        assert!(t.contains("[y] yes   [k] yes and allow keyboard mode   [Enter or any other key] deny"), "{t}");
        assert!(!t.contains("[s]"), "{t}");
    }

    #[tokio::test]
    async fn o_anuncio_de_ativar_o_teclado_em_ingles_nao_oferece_liberar() {
        let (c, acesso, _) = console();
        let pedido = Pedido { id: 0, tipo: Tipo::AtivarTeclado { nome: "iPhone".into() }, origem: None };
        let _tarefa = pedir(&acesso, pedido).await;
        let t = em_ingles(|| texto(&c.tique(T0)));
        assert!(t.contains("[y] yes   [Enter or any other key] deny"), "{t}");
        assert!(!t.contains("[k]"), "{t}");
    }

    #[test]
    fn o_tique_em_ingles_dica_o_fechamento_e_a_contagem() {
        let (c, acesso, _) = console();
        let dica = em_ingles(|| texto(&c.tique(T0)));
        assert!(dica.contains("pairing CLOSED: press Enter to open it for 2 minutes (c = certificate page, ? = help)"), "{dica}");
        acesso.pareamento().abrir(T0, 120);
        let aberto = em_ingles(|| texto(&c.tique(T0 + 1)));
        assert!(aberto.contains("pairing closes in"), "{aberto}");
        acesso.pareamento().fechar();
        let fechado = em_ingles(|| texto(&c.tique(T0 + 2)));
        assert!(fechado.contains("pairing CLOSED: press Enter to open it again"), "{fechado}");
    }

    #[test]
    fn a_pagina_do_certificado_em_ingles() {
        let (c, _, falso) = console();
        let aberta = em_ingles(|| texto(&c.linha("c", T0)));
        assert!(aberta.contains("certificate setup page open for 300 s: open on the phone http://192.168.68.103:8080/"), "{aberta}");
        *falso.falhar.lock().unwrap() = true;
        let falha = em_ingles(|| texto(&c.linha("c", T0 + 1)));
        assert!(falha.contains("could not open the setup page"), "{falha}");
    }
}
