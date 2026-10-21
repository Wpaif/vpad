//! Controle de acesso: só aparelhos pareados conectam. Para parear, o celular digita o
//! código de 6 dígitos que o terminal mostra (ver `pareamento`) e recebe um token só dele
//! (ver `dispositivos`). Vários aparelhos pareados podem conectar ao mesmo tempo.

use std::{io, net::IpAddr, sync::Arc, time::Duration};

use crate::{
    alerta::{Alerta, Alertas, ignorar},
    aprovacao::{Aprovacoes, Decisao, Ocupado, Pedido, Tipo},
    dispositivos::{Dispositivos, Pareado, Permissoes, limpar_nome},
    pareamento::{MAX_ERROS, Pareamento, Resultado},
};

/// Quanto tempo o PC espera pelo "sim" antes de desistir do pedido.
const PRAZO_DA_APROVACAO: Duration = Duration::from_secs(60);

pub struct Acesso {
    dispositivos: Dispositivos,
    pareamento: Pareamento,
    inatividade: Duration,
    alertas: Alertas,
    aprovacoes: Arc<Aprovacoes>,
    prazo_da_aprovacao: Duration,
}

/// O que aconteceu numa tentativa de pareamento.
pub enum Pareamento_ {
    Pareado(Pareado),
    Incorreto,
    /// o pareamento não está aberto no PC
    Fechado,
    /// quem está no PC negou o pedido
    Negado,
    /// ninguém aprovou o pedido a tempo
    Expirou,
    /// já há outro pedido de pareamento esperando aprovação
    Ocupado,
    Bloqueado { segundos: u64 },
    /// o código estava certo, mas não consegui gravar o cadastro
    Falha(io::Error),
}

impl std::fmt::Debug for Pareamento_ {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pareamento_::Pareado(_) => f.write_str("Pareado(..)"),
            Pareamento_::Incorreto => f.write_str("Incorreto"),
            Pareamento_::Fechado => f.write_str("Fechado"),
            Pareamento_::Negado => f.write_str("Negado"),
            Pareamento_::Expirou => f.write_str("Expirou"),
            Pareamento_::Ocupado => f.write_str("Ocupado"),
            Pareamento_::Bloqueado { segundos } => write!(f, "Bloqueado({segundos}s)"),
            Pareamento_::Falha(e) => write!(f, "Falha({e})"),
        }
    }
}

impl Acesso {
    pub fn novo(dispositivos: Dispositivos, pareamento: Pareamento, inatividade: Duration) -> Arc<Self> {
        Self::com_alertas(dispositivos, pareamento, inatividade, ignorar())
    }

    /// Como `novo`, mas avisando `alertas` de tentativas suspeitas (erros, bloqueios...).
    pub fn com_alertas(
        dispositivos: Dispositivos,
        pareamento: Pareamento,
        inatividade: Duration,
        alertas: Alertas,
    ) -> Arc<Self> {
        Self::com_opcoes(dispositivos, pareamento, inatividade, alertas, PRAZO_DA_APROVACAO)
    }

    /// Como `com_alertas`, escolhendo quanto o PC espera pela aprovação (os testes usam prazos curtos).
    pub fn com_opcoes(
        dispositivos: Dispositivos,
        pareamento: Pareamento,
        inatividade: Duration,
        alertas: Alertas,
        prazo_da_aprovacao: Duration,
    ) -> Arc<Self> {
        Arc::new(Self {
            dispositivos,
            pareamento,
            inatividade,
            alertas,
            aprovacoes: Aprovacoes::nova(),
            prazo_da_aprovacao,
        })
    }

    /// Pedidos que esperam o "sim" de quem está no PC.
    pub fn aprovacoes(&self) -> &Arc<Aprovacoes> {
        &self.aprovacoes
    }

    pub fn dispositivos(&self) -> &Dispositivos {
        &self.dispositivos
    }

    pub fn pareamento(&self) -> &Pareamento {
        &self.pareamento
    }

    /// Sem sinal do celular por este tempo, a conexão é considerada morta.
    pub fn inatividade(&self) -> Duration {
        self.inatividade
    }

    pub fn autorizado(&self, token: &str) -> bool {
        self.dispositivos.autorizado(token)
    }

    /// As permissões do aparelho dono do token, ou nada se ele não for pareado.
    pub fn permissoes(&self, token: &str) -> Option<Permissoes> {
        self.dispositivos.consultar(token)
    }

    /// Quanto tempo o PC espera por uma confirmação antes de desistir.
    pub fn prazo_da_aprovacao(&self) -> Duration {
        self.prazo_da_aprovacao
    }

    /// Troca o código digitado por um token novo, exclusivo do aparelho, **depois que quem
    /// está no PC aprovar**. `origem` é o IP de quem pediu (aparece no pedido e nos alertas).
    /// Cada tentativa concluída (aprovada, negada ou expirada) fecha o pareamento.
    pub async fn parear(&self, codigo: &str, nome: &str, agora: u64, origem: Option<IpAddr>) -> Pareamento_ {
        match self.pareamento.validar(codigo, agora) {
            Resultado::Fechado => {
                (self.alertas)(Alerta::PareamentoFechado { origem });
                Pareamento_::Fechado
            }
            // quem insiste durante o bloqueio não inunda o terminal: o aviso saiu no começo dele
            Resultado::Bloqueado { segundos } => Pareamento_::Bloqueado { segundos },
            Resultado::Incorreto => {
                match self.pareamento.bloqueio_restante(agora) {
                    // este foi o erro que fechou a conta: avisa dele e do bloqueio
                    Some(segundos) => {
                        (self.alertas)(Alerta::CodigoErrado { origem, erros: MAX_ERROS });
                        (self.alertas)(Alerta::Bloqueado { origem, segundos });
                    }
                    None => {
                        let erros = self.pareamento.erros_recentes(agora);
                        (self.alertas)(Alerta::CodigoErrado { origem, erros });
                    }
                }
                Pareamento_::Incorreto
            }
            Resultado::Aceito => {
                let pedido = Pedido { id: 0, tipo: Tipo::Pareamento { nome: limpar_nome(nome) }, origem };
                match self.aprovacoes.solicitar(pedido, self.prazo_da_aprovacao).await {
                    Err(Ocupado) => Pareamento_::Ocupado,
                    Ok(Decisao::Aprovado { teclado }) => {
                        self.pareamento.fechar();
                        match self.dispositivos.parear_com(nome, agora, teclado) {
                            Ok(pareado) => {
                                (self.alertas)(Alerta::Pareado {
                                    nome: self.dispositivos.nome_de(&pareado.id),
                                    id: pareado.id.clone(),
                                    origem,
                                });
                                Pareamento_::Pareado(pareado)
                            }
                            Err(e) => Pareamento_::Falha(e),
                        }
                    }
                    Ok(Decisao::Negado) => {
                        self.pareamento.fechar();
                        (self.alertas)(Alerta::PareamentoNegado { origem });
                        Pareamento_::Negado
                    }
                    Ok(Decisao::Expirou) => {
                        self.pareamento.fechar();
                        (self.alertas)(Alerta::PareamentoExpirou { origem });
                        Pareamento_::Expirou
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::aprovacao::Decisao;
    use std::sync::Mutex;

    const T0: u64 = 1_700_000_010;

    fn novo_acesso(prazo: Duration) -> (Arc<Acesso>, Arc<Mutex<Vec<Alerta>>>) {
        let vistos = Arc::new(Mutex::new(vec![]));
        let destino = vistos.clone();
        let a = Acesso::com_opcoes(
            Dispositivos::em_memoria(),
            Pareamento::com_segredo(b"segredo-de-teste-123".to_vec()),
            Duration::from_secs(6),
            Arc::new(move |al| destino.lock().unwrap().push(al)),
            prazo,
        );
        a.pareamento().abrir(T0, 100_000);
        (a, vistos)
    }

    fn acesso() -> Arc<Acesso> {
        novo_acesso(Duration::from_secs(30)).0
    }

    fn ip(s: &str) -> Option<std::net::IpAddr> {
        Some(s.parse().unwrap())
    }

    fn errado(certo: &str) -> &'static str {
        if certo == "000000" { "000001" } else { "000000" }
    }

    async fn esperar_pedido(a: &Acesso) -> u64 {
        for _ in 0..400 {
            if let Some(p) = a.aprovacoes().pendente() {
                return p.id;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("o pedido de aprovação não apareceu");
    }

    /// Pede o pareamento com `codigo` e responde `decisao` no "terminal".
    async fn parear_e_responder(
        a: &Arc<Acesso>,
        codigo: &str,
        nome: &str,
        agora: u64,
        origem: Option<std::net::IpAddr>,
        decisao: Decisao,
    ) -> Pareamento_ {
        let (a2, codigo, nome) = (a.clone(), codigo.to_string(), nome.to_string());
        let tarefa = tokio::spawn(async move { a2.parear(&codigo, &nome, agora, origem).await });
        let id = esperar_pedido(a).await;
        a.aprovacoes().responder(id, decisao);
        tarefa.await.unwrap()
    }

    #[test]
    fn sem_ninguem_pareado_nenhum_token_vale() {
        let a = acesso();
        assert!(!a.autorizado(""));
        assert!(!a.autorizado("qualquer"));
    }

    #[tokio::test]
    async fn o_codigo_certo_so_pareia_depois_que_o_pc_aprova() {
        let a = acesso();
        let codigo = a.pareamento().codigo_atual(T0);
        let a2 = a.clone();
        let tarefa = tokio::spawn(async move { a2.parear(&codigo, "iPhone", T0, ip("192.168.68.50")).await });

        let id = esperar_pedido(&a).await;
        let pedido = a.aprovacoes().pendente().unwrap();
        assert!(pedido.descricao().contains("iPhone") && pedido.descricao().contains("192.168.68.50"));
        assert!(a.dispositivos().listar().is_empty(), "antes do 'sim' não existe aparelho");

        a.aprovacoes().responder(id, Decisao::Aprovado { teclado: false });
        match tarefa.await.unwrap() {
            Pareamento_::Pareado(p) => {
                assert!(a.autorizado(&p.token));
                assert_eq!(a.dispositivos().listar()[0].nome, "iPhone");
            }
            outro => panic!("esperava parear, veio {outro:?}"),
        }
    }

    #[tokio::test]
    async fn o_nome_mostrado_ao_dono_do_pc_vem_sem_caracteres_de_controle() {
        let a = acesso();
        let (a2, codigo) = (a.clone(), a.pareamento().codigo_atual(T0));
        let tarefa = tokio::spawn(async move { a2.parear(&codigo, "\u{1b}[31mMALICIOSO\n\rLinha falsa", T0, None).await });
        let id = esperar_pedido(&a).await;
        let texto = a.aprovacoes().pendente().unwrap().descricao();
        assert!(!texto.contains('\u{1b}') && !texto.contains('\n') && !texto.contains('\r'), "{texto:?}");
        assert!(texto.contains("MALICIOSO"));
        a.aprovacoes().responder(id, Decisao::Negado);
        let _ = tarefa.await;
    }

    #[tokio::test]
    async fn a_aprovacao_decide_se_o_aparelho_nasce_com_o_modo_teclado() {
        let a = acesso();
        let r = parear_e_responder(&a, &a.pareamento().codigo_atual(T0), "iPhone", T0, None, Decisao::Aprovado { teclado: true }).await;
        let Pareamento_::Pareado(p) = r else { panic!("{r:?}") };
        assert!(a.dispositivos().teclado_permitido(&p.token));

        a.pareamento().abrir(T0 + 30, 100_000); // o primeiro pareamento fechou a janela: o PC a reabre
        let r = parear_e_responder(&a, &a.pareamento().codigo_atual(T0 + 30), "iPad", T0 + 30, None, Decisao::Aprovado { teclado: false }).await;
        let Pareamento_::Pareado(p) = r else { panic!("{r:?}") };
        assert!(!a.dispositivos().teclado_permitido(&p.token));
    }

    #[tokio::test]
    async fn negar_no_pc_nao_cadastra_e_fecha_o_pareamento() {
        let (a, vistos) = novo_acesso(Duration::from_secs(30));
        let r = parear_e_responder(&a, &a.pareamento().codigo_atual(T0), "intruso", T0, ip("10.0.0.9"), Decisao::Negado).await;
        assert!(matches!(r, Pareamento_::Negado));
        assert!(a.dispositivos().listar().is_empty());
        assert!(!a.pareamento().aberto(T0 + 1), "depois de uma tentativa concluída o pareamento volta a fechar");
        assert_eq!(*vistos.lock().unwrap(), vec![Alerta::PareamentoNegado { origem: ip("10.0.0.9") }]);
    }

    #[tokio::test]
    async fn sem_resposta_no_pc_o_pedido_expira_e_fecha_o_pareamento() {
        let (a, vistos) = novo_acesso(Duration::from_millis(80));
        let codigo = a.pareamento().codigo_atual(T0);
        let r = a.parear(&codigo, "iPhone", T0, ip("10.0.0.9")).await;
        assert!(matches!(r, Pareamento_::Expirou));
        assert!(a.dispositivos().listar().is_empty());
        assert!(!a.pareamento().aberto(T0 + 1));
        assert_eq!(*vistos.lock().unwrap(), vec![Alerta::PareamentoExpirou { origem: ip("10.0.0.9") }]);
    }

    #[tokio::test]
    async fn parear_com_sucesso_fecha_o_pareamento_e_avisa() {
        let (a, vistos) = novo_acesso(Duration::from_secs(30));
        let r = parear_e_responder(&a, &a.pareamento().codigo_atual(T0), "iPhone", T0, ip("10.0.0.9"), Decisao::Aprovado { teclado: false }).await;
        let Pareamento_::Pareado(p) = r else { panic!("{r:?}") };
        assert!(!a.pareamento().aberto(T0 + 1), "uma abertura vale um pareamento");
        let alertas = vistos.lock().unwrap();
        assert_eq!(alertas.len(), 1, "{alertas:?}");
        assert_eq!(alertas[0], Alerta::Pareado { nome: "iPhone".into(), id: p.id, origem: ip("10.0.0.9") });
    }

    #[tokio::test]
    async fn segundo_pedido_enquanto_um_espera_resposta_recebe_ocupado_e_a_janela_segue_aberta() {
        let a = acesso();
        let (a2, codigo1) = (a.clone(), a.pareamento().codigo_atual(T0));
        let primeiro = tokio::spawn(async move { a2.parear(&codigo1, "iPhone", T0, None).await });
        let id = esperar_pedido(&a).await;

        let codigo2 = a.pareamento().codigo_atual(T0 + 30);
        assert!(matches!(a.parear(&codigo2, "iPad", T0 + 30, None).await, Pareamento_::Ocupado));
        assert!(a.pareamento().aberto(T0 + 31), "o pedido que sobrou não fecha a janela");
        assert!(a.dispositivos().listar().is_empty());

        a.aprovacoes().responder(id, Decisao::Negado);
        assert!(matches!(primeiro.await.unwrap(), Pareamento_::Negado));
    }

    #[tokio::test]
    async fn se_o_celular_desiste_durante_a_espera_o_pedido_some_sem_cadastrar_nada() {
        let a = acesso();
        let (a2, codigo) = (a.clone(), a.pareamento().codigo_atual(T0));
        let tarefa = tokio::spawn(async move { a2.parear(&codigo, "iPhone", T0, None).await });
        esperar_pedido(&a).await;
        tarefa.abort();
        let _ = tarefa.await;
        assert!(a.aprovacoes().pendente().is_none());
        assert!(a.dispositivos().listar().is_empty());
    }

    // ---------- erros, janela fechada e alertas ----------

    #[tokio::test]
    async fn codigo_errado_nao_pareia_nem_cadastra_nem_pede_aprovacao() {
        let a = acesso();
        let errado = errado(&a.pareamento().codigo_atual(T0));
        assert!(matches!(a.parear(errado, "iPhone", T0, None).await, Pareamento_::Incorreto));
        assert!(a.dispositivos().listar().is_empty());
        assert!(a.aprovacoes().pendente().is_none());
    }

    #[tokio::test]
    async fn muitos_erros_bloqueiam_ate_o_codigo_certo() {
        let a = acesso();
        let certo = a.pareamento().codigo_atual(T0);
        for _ in 0..5 {
            a.parear(errado(&certo), "x", T0, None).await;
        }
        assert!(matches!(a.parear(&certo, "x", T0 + 1, None).await, Pareamento_::Bloqueado { .. }));
        assert!(a.dispositivos().listar().is_empty());
    }

    #[tokio::test]
    async fn com_a_janela_fechada_nem_o_codigo_certo_pareia_e_o_alerta_diz_de_onde_veio() {
        let (a, vistos) = novo_acesso(Duration::from_secs(30));
        a.pareamento().fechar();
        let codigo = a.pareamento().codigo_atual(T0);
        assert!(matches!(a.parear(&codigo, "x", T0, ip("192.168.68.77")).await, Pareamento_::Fechado));
        assert!(a.dispositivos().listar().is_empty());
        assert_eq!(*vistos.lock().unwrap(), vec![Alerta::PareamentoFechado { origem: ip("192.168.68.77") }]);
    }

    #[tokio::test]
    async fn cada_codigo_errado_gera_um_alerta_com_a_origem_e_a_contagem() {
        let (a, vistos) = novo_acesso(Duration::from_secs(30));
        let errado = errado(&a.pareamento().codigo_atual(T0));
        a.parear(errado, "x", T0, ip("10.0.0.9")).await;
        a.parear(errado, "x", T0 + 1, ip("10.0.0.9")).await;
        assert_eq!(
            *vistos.lock().unwrap(),
            vec![
                Alerta::CodigoErrado { origem: ip("10.0.0.9"), erros: 1 },
                Alerta::CodigoErrado { origem: ip("10.0.0.9"), erros: 2 },
            ]
        );
    }

    #[tokio::test]
    async fn o_quinto_erro_avisa_do_bloqueio_uma_vez_so() {
        let (a, vistos) = novo_acesso(Duration::from_secs(30));
        let errado = errado(&a.pareamento().codigo_atual(T0));
        for i in 0..5 {
            a.parear(errado, "x", T0 + i, ip("10.0.0.9")).await;
        }
        {
            let alertas = vistos.lock().unwrap();
            assert_eq!(alertas.len(), 6, "5 erros + o aviso de bloqueio: {alertas:?}");
            assert_eq!(alertas[4], Alerta::CodigoErrado { origem: ip("10.0.0.9"), erros: 5 });
            assert_eq!(alertas[5], Alerta::Bloqueado { origem: ip("10.0.0.9"), segundos: 60 });
        }
        // insistir durante o bloqueio não inunda o terminal
        for i in 5..15 {
            assert!(matches!(a.parear(errado, "x", T0 + i, ip("10.0.0.9")).await, Pareamento_::Bloqueado { .. }));
        }
        assert_eq!(vistos.lock().unwrap().len(), 6);
    }

    #[test]
    fn expoe_o_tempo_de_inatividade() {
        assert_eq!(acesso().inatividade(), Duration::from_secs(6));
    }
}
