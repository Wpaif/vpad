//! Controle de acesso: só aparelhos pareados conectam. Para parear, o celular digita o
//! código de 6 dígitos que o terminal mostra (ver `pareamento`) e recebe um token só dele
//! (ver `dispositivos`). Vários aparelhos pareados podem conectar ao mesmo tempo.

use std::{io, sync::Arc, time::Duration};

use crate::{
    dispositivos::{Dispositivos, Pareado},
    pareamento::{Pareamento, Resultado},
};

pub struct Acesso {
    dispositivos: Dispositivos,
    pareamento: Pareamento,
    inatividade: Duration,
}

/// O que aconteceu numa tentativa de pareamento.
pub enum Pareamento_ {
    Pareado(Pareado),
    Incorreto,
    Bloqueado { segundos: u64 },
    /// o código estava certo, mas não consegui gravar o cadastro
    Falha(io::Error),
}

impl std::fmt::Debug for Pareamento_ {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pareamento_::Pareado(_) => f.write_str("Pareado(..)"),
            Pareamento_::Incorreto => f.write_str("Incorreto"),
            Pareamento_::Bloqueado { segundos } => write!(f, "Bloqueado({segundos}s)"),
            Pareamento_::Falha(e) => write!(f, "Falha({e})"),
        }
    }
}

impl Acesso {
    pub fn novo(dispositivos: Dispositivos, pareamento: Pareamento, inatividade: Duration) -> Arc<Self> {
        Arc::new(Self { dispositivos, pareamento, inatividade })
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

    /// Troca o código digitado por um token novo, exclusivo do aparelho.
    pub fn parear(&self, codigo: &str, nome: &str, agora: u64) -> Pareamento_ {
        match self.pareamento.validar(codigo, agora) {
            Resultado::Incorreto => Pareamento_::Incorreto,
            Resultado::Bloqueado { segundos } => Pareamento_::Bloqueado { segundos },
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

    fn acesso() -> Arc<Acesso> {
        Acesso::novo(
            Dispositivos::em_memoria(),
            Pareamento::com_segredo(b"segredo-de-teste-123".to_vec()),
            Duration::from_secs(6),
        )
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
        match a.parear(&codigo, "iPhone", T0) {
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
        assert!(matches!(a.parear(errado, "iPhone", T0), Pareamento_::Incorreto));
        assert!(a.dispositivos().listar().is_empty());
    }

    #[test]
    fn muitos_erros_bloqueiam_ate_o_codigo_certo() {
        let a = acesso();
        let certo = a.pareamento().codigo_atual(T0);
        let errado = if certo == "000000" { "000001" } else { "000000" };
        for _ in 0..5 {
            a.parear(errado, "x", T0);
        }
        assert!(matches!(a.parear(&certo, "x", T0 + 1), Pareamento_::Bloqueado { .. }));
        assert!(a.dispositivos().listar().is_empty());
    }

    #[test]
    fn dois_aparelhos_pareiam_cada_um_com_o_seu_token() {
        let a = acesso();
        let Pareamento_::Pareado(um) = a.parear(&a.pareamento().codigo_atual(T0), "iPhone", T0) else {
            panic!("primeiro")
        };
        // o código é de uso único: o segundo aparelho espera o próximo
        let Pareamento_::Pareado(dois) = a.parear(&a.pareamento().codigo_atual(T0 + 30), "iPad", T0 + 30) else {
            panic!("segundo")
        };
        assert_ne!(um.token, dois.token);
        assert!(a.autorizado(&um.token) && a.autorizado(&dois.token));
    }

    #[test]
    fn expoe_o_tempo_de_inatividade() {
        assert_eq!(acesso().inatividade(), Duration::from_secs(6));
    }
}
