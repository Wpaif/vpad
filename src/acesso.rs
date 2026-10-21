//! Controle de acesso: só aparelhos pareados conectam. Para parear, o celular digita o
//! código de 6 dígitos que o terminal mostra (ver `pareamento`) e recebe um token só dele
//! (ver `dispositivos`). Vários aparelhos pareados podem conectar ao mesmo tempo.

use std::{io, net::IpAddr, sync::Arc, time::Duration};

use crate::{
    alerta::{Alerta, Alertas, ignorar},
    dispositivos::{Dispositivos, Pareado},
    pareamento::{MAX_ERROS, Pareamento, Resultado},
};

pub struct Acesso {
    dispositivos: Dispositivos,
    pareamento: Pareamento,
    inatividade: Duration,
    alertas: Alertas,
}

/// O que aconteceu numa tentativa de pareamento.
pub enum Pareamento_ {
    Pareado(Pareado),
    Incorreto,
    /// o pareamento não está aberto no PC
    Fechado,
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
        Arc::new(Self { dispositivos, pareamento, inatividade, alertas })
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

    /// Troca o código digitado por um token novo, exclusivo do aparelho. `origem` é o IP de
    /// quem pediu, usado só nos alertas.
    pub fn parear(&self, codigo: &str, nome: &str, agora: u64, origem: Option<IpAddr>) -> Pareamento_ {
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
            Resultado::Aceito => match self.dispositivos.parear(nome, agora) {
                Ok(pareado) => Pareamento_::Pareado(pareado),
                Err(e) => Pareamento_::Falha(e),
            },
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const T0: u64 = 1_700_000_010;

    use crate::alerta::Alerta;
    use std::sync::Mutex;

    fn acesso() -> Arc<Acesso> {
        let a = Acesso::novo(
            Dispositivos::em_memoria(),
            Pareamento::com_segredo(b"segredo-de-teste-123".to_vec()),
            Duration::from_secs(6),
        );
        a.pareamento().abrir(T0, 100_000);
        a
    }

    /// Acesso com a janela aberta e um coletor dos alertas emitidos.
    fn acesso_com_alertas() -> (Arc<Acesso>, Arc<Mutex<Vec<Alerta>>>) {
        let vistos = Arc::new(Mutex::new(vec![]));
        let destino = vistos.clone();
        let a = Acesso::com_alertas(
            Dispositivos::em_memoria(),
            Pareamento::com_segredo(b"segredo-de-teste-123".to_vec()),
            Duration::from_secs(6),
            Arc::new(move |al| destino.lock().unwrap().push(al)),
        );
        a.pareamento().abrir(T0, 100_000);
        (a, vistos)
    }

    fn ip(s: &str) -> Option<std::net::IpAddr> {
        Some(s.parse().unwrap())
    }

    fn errado(certo: &str) -> &'static str {
        if certo == "000000" { "000001" } else { "000000" }
    }

    #[test]
    fn sem_ninguem_pareado_nenhum_token_vale() {
        let a = acesso();
        assert!(!a.autorizado(""));
        assert!(!a.autorizado("qualquer"));
    }

    #[test]
    fn o_codigo_certo_pareia_e_devolve_um_token_que_passa_a_valer() {
        let a = acesso();
        let codigo = a.pareamento().codigo_atual(T0);
        match a.parear(&codigo, "iPhone", T0, None) {
            Pareamento_::Pareado(p) => {
                assert!(a.autorizado(&p.token));
                assert_eq!(a.dispositivos().listar().len(), 1);
                assert_eq!(a.dispositivos().listar()[0].nome, "iPhone");
            }
            outro => panic!("esperava parear, veio {outro:?}"),
        }
    }

    #[test]
    fn codigo_errado_nao_pareia_nem_cadastra() {
        let a = acesso();
        let certo = a.pareamento().codigo_atual(T0);
        let errado = if certo == "000000" { "000001" } else { "000000" };
        assert!(matches!(a.parear(errado, "iPhone", T0, None), Pareamento_::Incorreto));
        assert!(a.dispositivos().listar().is_empty());
    }

    #[test]
    fn muitos_erros_bloqueiam_ate_o_codigo_certo() {
        let a = acesso();
        let certo = a.pareamento().codigo_atual(T0);
        let errado = if certo == "000000" { "000001" } else { "000000" };
        for _ in 0..5 {
            a.parear(errado, "x", T0, None);
        }
        assert!(matches!(a.parear(&certo, "x", T0 + 1, None), Pareamento_::Bloqueado { .. }));
        assert!(a.dispositivos().listar().is_empty());
    }

    #[test]
    fn dois_aparelhos_pareiam_cada_um_com_o_seu_token() {
        let a = acesso();
        let Pareamento_::Pareado(um) = a.parear(&a.pareamento().codigo_atual(T0), "iPhone", T0, None) else {
            panic!("primeiro")
        };
        // o código é de uso único: o segundo aparelho espera o próximo
        let Pareamento_::Pareado(dois) = a.parear(&a.pareamento().codigo_atual(T0 + 30), "iPad", T0 + 30, None) else {
            panic!("segundo")
        };
        assert_ne!(um.token, dois.token);
        assert!(a.autorizado(&um.token) && a.autorizado(&dois.token));
    }

    #[test]
    fn expoe_o_tempo_de_inatividade() {
        assert_eq!(acesso().inatividade(), Duration::from_secs(6));
    }

    // ---------- janela fechada e alertas ----------

    #[test]
    fn com_a_janela_fechada_nem_o_codigo_certo_pareia_e_o_alerta_diz_de_onde_veio() {
        let (a, vistos) = acesso_com_alertas();
        a.pareamento().fechar();
        let codigo = a.pareamento().codigo_atual(T0);
        assert!(matches!(a.parear(&codigo, "x", T0, ip("192.168.68.77")), Pareamento_::Fechado));
        assert!(a.dispositivos().listar().is_empty());
        assert_eq!(*vistos.lock().unwrap(), vec![Alerta::PareamentoFechado { origem: ip("192.168.68.77") }]);
    }

    #[test]
    fn cada_codigo_errado_gera_um_alerta_com_a_origem_e_a_contagem() {
        let (a, vistos) = acesso_com_alertas();
        let errado = errado(&a.pareamento().codigo_atual(T0));
        a.parear(errado, "x", T0, ip("10.0.0.9"));
        a.parear(errado, "x", T0 + 1, ip("10.0.0.9"));
        assert_eq!(
            *vistos.lock().unwrap(),
            vec![
                Alerta::CodigoErrado { origem: ip("10.0.0.9"), erros: 1 },
                Alerta::CodigoErrado { origem: ip("10.0.0.9"), erros: 2 },
            ]
        );
    }

    #[test]
    fn o_quinto_erro_avisa_do_bloqueio_uma_vez_so() {
        let (a, vistos) = acesso_com_alertas();
        let errado = errado(&a.pareamento().codigo_atual(T0));
        for i in 0..5 {
            a.parear(errado, "x", T0 + i, ip("10.0.0.9"));
        }
        {
            let alertas = vistos.lock().unwrap();
            assert_eq!(alertas.len(), 6, "5 erros + o aviso de bloqueio: {alertas:?}");
            assert_eq!(alertas[4], Alerta::CodigoErrado { origem: ip("10.0.0.9"), erros: 5 });
            assert_eq!(alertas[5], Alerta::Bloqueado { origem: ip("10.0.0.9"), segundos: 60 });
        }
        // insistir durante o bloqueio não inunda o terminal
        for i in 5..15 {
            assert!(matches!(a.parear(errado, "x", T0 + i, ip("10.0.0.9")), Pareamento_::Bloqueado { .. }));
        }
        assert_eq!(vistos.lock().unwrap().len(), 6);
    }

    #[test]
    fn parear_com_sucesso_nao_gera_alerta_de_erro() {
        let (a, vistos) = acesso_com_alertas();
        let codigo = a.pareamento().codigo_atual(T0);
        assert!(matches!(a.parear(&codigo, "iPhone", T0, ip("10.0.0.9")), Pareamento_::Pareado(_)));
        assert!(vistos.lock().unwrap().is_empty());
    }
}
