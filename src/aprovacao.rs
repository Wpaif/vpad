//! Aprovações no PC: um pedido (parear um aparelho novo, ativar o modo teclado) só segue
//! se quem está diante do terminal confirmar. Só há um pedido pendente por vez, com prazo,
//! e ele some sozinho se o celular desistir.

use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::sync::{oneshot, watch};
use crate::{mensagens::txt, tr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tipo {
    /// um aparelho novo quer ser pareado (já digitou o código certo)
    Pareamento { nome: String },
    /// um aparelho pareado quer ativar o modo teclado, que digita no PC
    AtivarTeclado { nome: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pedido {
    pub id: u64,
    pub tipo: Tipo,
    pub origem: Option<IpAddr>,
}

impl Pedido {
    /// O texto do pedido para o terminal.
    pub fn descricao(&self) -> String {
        let de = self.origem.map_or_else(|| txt("origem desconhecida", "unknown origin").to_string(), |ip| ip.to_string());
        match &self.tipo {
            Tipo::Pareamento { nome } => tr!("parear o aparelho '{nome}' ({de})", "pair the device '{nome}' ({de})"),
            Tipo::AtivarTeclado { nome } => tr!(
                "ativar o MODO TECLADO do aparelho '{nome}' ({de}): ele passará a digitar no PC",
                "enable KEYBOARD MODE on the device '{nome}' ({de}): it will type on the PC"
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decisao {
    /// `teclado`: no pareamento, também liberar o modo teclado para o aparelho
    Aprovado { teclado: bool },
    Negado,
    /// ninguém respondeu no prazo
    Expirou,
}

/// Já há outro pedido esperando resposta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ocupado;

struct Pendente {
    id: u64,
    resposta: oneshot::Sender<Decisao>,
}

#[derive(Default)]
struct Estado {
    proximo_id: u64,
    atual: Option<Pendente>,
}

pub struct Aprovacoes {
    estado: Mutex<Estado>,
    publicado: watch::Sender<Option<Pedido>>,
}

/// Limpa o pedido se quem esperava a resposta sumir (timeout ou requisição cancelada).
struct Guarda {
    aprovacoes: Arc<Aprovacoes>,
    id: u64,
}

impl Drop for Guarda {
    fn drop(&mut self) {
        let mut estado = self.aprovacoes.estado.lock().unwrap();
        if estado.atual.as_ref().is_some_and(|p| p.id == self.id) {
            estado.atual = None;
            self.aprovacoes.publicado.send_replace(None);
        }
    }
}

impl Aprovacoes {
    pub fn nova() -> Arc<Self> {
        Arc::new(Self { estado: Mutex::new(Estado::default()), publicado: watch::channel(None).0 })
    }

    /// O pedido que espera resposta agora, se houver.
    pub fn pendente(&self) -> Option<Pedido> {
        self.publicado.borrow().clone()
    }

    /// Para o terminal saber quando um pedido chega e quando ele acaba.
    pub fn observar(&self) -> watch::Receiver<Option<Pedido>> {
        self.publicado.subscribe()
    }

    /// Responde ao pedido `id`. Devolve false se ele não está mais pendente (expirou, foi
    /// cancelado ou o id é de um pedido antigo): a resposta nunca cai no pedido seguinte.
    pub fn responder(&self, id: u64, decisao: Decisao) -> bool {
        let mut estado = self.estado.lock().unwrap();
        if !estado.atual.as_ref().is_some_and(|p| p.id == id) {
            return false;
        }
        let pendente = estado.atual.take().expect("conferido acima");
        self.publicado.send_replace(None);
        pendente.resposta.send(decisao).is_ok()
    }

    /// Pede a aprovação e espera até `prazo`. Se a tarefa que chama for cancelada, o pedido
    /// some junto.
    pub async fn solicitar(self: &Arc<Self>, pedido: Pedido, prazo: Duration) -> Result<Decisao, Ocupado> {
        let (id, receber) = {
            let mut estado = self.estado.lock().unwrap();
            if estado.atual.is_some() {
                return Err(Ocupado);
            }
            estado.proximo_id += 1;
            let id = estado.proximo_id;
            let (enviar, receber) = oneshot::channel();
            estado.atual = Some(Pendente { id, resposta: enviar });
            self.publicado.send_replace(Some(Pedido { id, ..pedido }));
            (id, receber)
        };
        let _guarda = Guarda { aprovacoes: self.clone(), id };
        Ok(match tokio::time::timeout(prazo, receber).await {
            Ok(Ok(decisao)) => decisao,
            Ok(Err(_)) | Err(_) => Decisao::Expirou,
        })
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::time::Duration;

    const PRAZO: Duration = Duration::from_secs(5);

    fn pareamento(nome: &str) -> Pedido {
        Pedido { tipo: Tipo::Pareamento { nome: nome.into() }, origem: Some("192.168.68.50".parse().unwrap()), id: 0 }
    }

    fn teclado(nome: &str) -> Pedido {
        Pedido { tipo: Tipo::AtivarTeclado { nome: nome.into() }, origem: None, id: 0 }
    }

    /// Espera o pedido aparecer como pendente e devolve o id dele.
    async fn esperar_pendente(a: &Aprovacoes) -> u64 {
        for _ in 0..200 {
            if let Some(p) = a.pendente() {
                return p.id;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("o pedido não apareceu como pendente");
    }

    #[tokio::test]
    async fn aprovar_devolve_a_decisao_ao_solicitante() {
        let a = Aprovacoes::nova();
        let a2 = a.clone();
        let tarefa = tokio::spawn(async move { a2.solicitar(pareamento("iPhone"), PRAZO).await });
        let id = esperar_pendente(&a).await;

        assert!(a.responder(id, Decisao::Aprovado { teclado: false }));
        assert_eq!(tarefa.await.unwrap(), Ok(Decisao::Aprovado { teclado: false }));
    }

    #[tokio::test]
    async fn aprovar_pode_vir_com_a_permissao_de_teclado() {
        let a = Aprovacoes::nova();
        let a2 = a.clone();
        let tarefa = tokio::spawn(async move { a2.solicitar(pareamento("iPhone"), PRAZO).await });
        let id = esperar_pendente(&a).await;
        a.responder(id, Decisao::Aprovado { teclado: true });
        assert_eq!(tarefa.await.unwrap(), Ok(Decisao::Aprovado { teclado: true }));
    }

    #[tokio::test]
    async fn negar_devolve_negado() {
        let a = Aprovacoes::nova();
        let a2 = a.clone();
        let tarefa = tokio::spawn(async move { a2.solicitar(pareamento("iPhone"), PRAZO).await });
        let id = esperar_pendente(&a).await;
        assert!(a.responder(id, Decisao::Negado));
        assert_eq!(tarefa.await.unwrap(), Ok(Decisao::Negado));
    }

    #[tokio::test]
    async fn sem_resposta_no_prazo_expira() {
        let a = Aprovacoes::nova();
        let r = a.solicitar(pareamento("iPhone"), Duration::from_millis(60)).await;
        assert_eq!(r, Ok(Decisao::Expirou));
        assert!(a.pendente().is_none(), "depois de expirar não sobra pedido pendente");
    }

    #[tokio::test]
    async fn so_um_pedido_por_vez() {
        let a = Aprovacoes::nova();
        let a2 = a.clone();
        let primeiro = tokio::spawn(async move { a2.solicitar(pareamento("iPhone"), PRAZO).await });
        let id = esperar_pendente(&a).await;

        assert_eq!(a.solicitar(pareamento("outro"), PRAZO).await, Err(Ocupado));
        assert_eq!(a.solicitar(teclado("outro"), PRAZO).await, Err(Ocupado));

        a.responder(id, Decisao::Negado);
        primeiro.await.unwrap().unwrap();
        // liberou: um novo pedido já pode ser feito
        let a3 = a.clone();
        let segundo = tokio::spawn(async move { a3.solicitar(teclado("iPad"), PRAZO).await });
        let id2 = esperar_pendente(&a).await;
        assert_ne!(id, id2, "cada pedido tem o seu id");
        a.responder(id2, Decisao::Aprovado { teclado: false });
        assert_eq!(segundo.await.unwrap(), Ok(Decisao::Aprovado { teclado: false }));
    }

    #[tokio::test]
    async fn expone_o_pedido_pendente_para_o_terminal_mostrar() {
        let a = Aprovacoes::nova();
        let a2 = a.clone();
        let tarefa = tokio::spawn(async move { a2.solicitar(pareamento("iPhone do Wilian"), PRAZO).await });
        let id = esperar_pendente(&a).await;

        let p = a.pendente().unwrap();
        assert_eq!(p.id, id);
        assert_eq!(p.origem, Some("192.168.68.50".parse().unwrap()));
        let texto = p.descricao();
        assert!(texto.contains("iPhone do Wilian") && texto.contains("192.168.68.50"), "{texto}");

        a.responder(id, Decisao::Negado);
        tarefa.await.unwrap().unwrap();
        assert!(a.pendente().is_none());
    }

    #[tokio::test]
    async fn a_descricao_distingue_parear_de_ativar_o_teclado() {
        let parear = pareamento("iPhone").descricao().to_lowercase();
        let ativar = teclado("iPhone").descricao().to_lowercase();
        assert!(parear.contains("parear"), "{parear}");
        assert!(ativar.contains("teclado") && ativar.contains("digit"), "{ativar}");
        assert!(ativar.contains("origem desconhecida") || !ativar.contains("192."), "{ativar}");
    }

    #[tokio::test]
    async fn responder_sem_pedido_ou_com_id_velho_nao_faz_nada() {
        let a = Aprovacoes::nova();
        assert!(!a.responder(1, Decisao::Negado), "sem pedido pendente");

        let a2 = a.clone();
        let tarefa = tokio::spawn(async move { a2.solicitar(pareamento("iPhone"), Duration::from_millis(60)).await });
        let velho = esperar_pendente(&a).await;
        assert_eq!(tarefa.await.unwrap(), Ok(Decisao::Expirou));

        // o usuário digita "s" tarde demais: a resposta não pode cair no pedido seguinte
        let a3 = a.clone();
        let novo = tokio::spawn(async move { a3.solicitar(teclado("iPad"), PRAZO).await });
        let id_novo = esperar_pendente(&a).await;
        assert!(!a.responder(velho, Decisao::Aprovado { teclado: true }));
        assert!(a.pendente().is_some());
        a.responder(id_novo, Decisao::Negado);
        assert_eq!(novo.await.unwrap(), Ok(Decisao::Negado));
    }

    #[tokio::test]
    async fn se_o_celular_desiste_o_pedido_some() {
        let a = Aprovacoes::nova();
        let a2 = a.clone();
        let tarefa = tokio::spawn(async move { a2.solicitar(pareamento("iPhone"), PRAZO).await });
        let id = esperar_pendente(&a).await;

        tarefa.abort(); // a conexão do celular caiu: a requisição HTTP foi cancelada
        let _ = tarefa.await;
        assert!(a.pendente().is_none());
        assert!(!a.responder(id, Decisao::Aprovado { teclado: false }), "ninguém mais espera a resposta");
    }

    #[tokio::test]
    async fn o_terminal_e_avisado_quando_o_pedido_chega_e_quando_acaba() {
        let a = Aprovacoes::nova();
        let mut observador = a.observar();
        assert!(observador.borrow().is_none());

        let a2 = a.clone();
        let tarefa = tokio::spawn(async move { a2.solicitar(pareamento("iPhone"), PRAZO).await });
        observador.changed().await.unwrap();
        let id = observador.borrow_and_update().as_ref().unwrap().id;

        a.responder(id, Decisao::Negado);
        tarefa.await.unwrap().unwrap();
        observador.changed().await.unwrap();
        assert!(observador.borrow_and_update().is_none());
    }
}

#[cfg(test)]
mod ingles {
    use super::*;
    use crate::{idiomas::Idioma, mensagens::com_idioma};

    fn pedido(tipo: Tipo, origem: Option<&str>) -> Pedido {
        Pedido { id: 1, tipo, origem: origem.map(|o| o.parse().unwrap()) }
    }

    #[test]
    fn descricao_dos_pedidos_em_ingles() {
        com_idioma(Idioma::En, || {
            let p = pedido(Tipo::Pareamento { nome: "iPhone".into() }, Some("192.168.68.50"));
            assert_eq!(p.descricao(), "pair the device 'iPhone' (192.168.68.50)");
            let t = pedido(Tipo::AtivarTeclado { nome: "iPhone".into() }, None);
            assert_eq!(t.descricao(), "enable KEYBOARD MODE on the device 'iPhone' (unknown origin): it will type on the PC");
        });
    }

    #[test]
    fn descricao_em_portugues_continua_igual() {
        let p = pedido(Tipo::Pareamento { nome: "iPhone".into() }, None);
        assert_eq!(p.descricao(), "parear o aparelho 'iPhone' (origem desconhecida)");
    }
}
