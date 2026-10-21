//! Pareamento de um celular novo por um código de 6 dígitos que muda a cada 30 s (ver `totp`).
//! O código aparece só no terminal do PC. Como 6 dígitos são poucos, cada código vale uma
//! única vez e erros em excesso bloqueiam o pareamento por um tempo.

use std::{
    collections::VecDeque,
    fs,
    io::{self, Read},
    sync::Mutex,
};

use crate::totp;

/// Erros tolerados dentro de `JANELA_DE_ERROS` segundos antes de bloquear.
const MAX_ERROS: usize = 5;
const JANELA_DE_ERROS: u64 = 60;

#[derive(Debug, PartialEq, Eq)]
pub enum Resultado {
    Aceito,
    Incorreto,
    /// muitas tentativas erradas: nem o código certo é aceito até passar o tempo
    Bloqueado { segundos: u64 },
}

pub struct Pareamento {
    segredo: Vec<u8>,
    estado: Mutex<Estado>,
}

#[derive(Default)]
struct Estado {
    /// último intervalo cujo código já foi usado: ele e os anteriores não valem mais
    ultimo_usado: Option<u64>,
    /// instantes (segundos) dos erros recentes
    erros: VecDeque<u64>,
}

/// Compara em tempo constante, para o tempo de resposta não revelar quantos dígitos batem.
pub(crate) fn iguais(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diferenca = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        diferenca |= (a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0)) as usize;
    }
    diferenca == 0
}

impl Pareamento {
    /// Segredo novo e aleatório (160 bits), que vive só enquanto o servidor roda.
    pub fn novo() -> io::Result<Self> {
        let mut segredo = vec![0u8; 20];
        fs::File::open("/dev/urandom")?.read_exact(&mut segredo)?;
        Ok(Self::com_segredo(segredo))
    }

    pub fn com_segredo(segredo: Vec<u8>) -> Self {
        Self { segredo, estado: Mutex::new(Estado::default()) }
    }

    /// O código que o terminal mostra agora.
    pub fn codigo_atual(&self, agora: u64) -> String {
        totp::codigo(&self.segredo, totp::passo(agora))
    }

    pub fn segundos_restantes(&self, agora: u64) -> u64 {
        totp::segundos_restantes(agora)
    }

    /// Confere o código digitado. Vale o do intervalo atual e o do anterior (quem digita
    /// devagar não perde o código por um segundo), uma única vez cada um.
    pub fn validar(&self, informado: &str, agora: u64) -> Resultado {
        let mut estado = self.estado.lock().unwrap();
        while estado.erros.front().is_some_and(|t| agora.saturating_sub(*t) >= JANELA_DE_ERROS) {
            estado.erros.pop_front();
        }
        if estado.erros.len() >= MAX_ERROS {
            let liberado_em = estado.erros.front().copied().unwrap_or(agora) + JANELA_DE_ERROS;
            return Resultado::Bloqueado { segundos: liberado_em.saturating_sub(agora).max(1) };
        }

        let atual = totp::passo(agora);
        let mut aceito = None;
        for passo in [Some(atual), atual.checked_sub(1)].into_iter().flatten() {
            let vale = estado.ultimo_usado.is_none_or(|usado| passo > usado);
            // compara sempre, mesmo sem valer, para o tempo não depender do resultado
            let bate = iguais(&totp::codigo(&self.segredo, passo), informado);
            if bate && vale && aceito.is_none() {
                aceito = Some(passo);
            }
        }

        match aceito {
            Some(passo) => {
                estado.ultimo_usado = Some(passo);
                estado.erros.clear();
                Resultado::Aceito
            }
            None => {
                estado.erros.push_back(agora);
                Resultado::Incorreto
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const T0: u64 = 1_700_000_010; // início de um intervalo de 30 s + 10 s

    fn p() -> Pareamento {
        Pareamento::com_segredo(b"segredo-de-teste-123".to_vec())
    }

    /// Um código de 6 dígitos diferente de `certo`.
    fn errado(certo: &str) -> String {
        if certo == "000000" { "000001".into() } else { "000000".into() }
    }

    #[test]
    fn o_codigo_do_terminal_pareia() {
        let p = p();
        assert_eq!(p.validar(&p.codigo_atual(T0), T0), Resultado::Aceito);
    }

    #[test]
    fn codigo_errado_e_incorreto() {
        let p = p();
        let certo = p.codigo_atual(T0);
        assert_eq!(p.validar(&errado(&certo), T0), Resultado::Incorreto);
    }

    #[test]
    fn entradas_malformadas_sao_incorretas() {
        for entrada in ["", "12345", "1234567", "abcdef", "12 345", "１２３４５６"] {
            // uma instância por entrada: seis erros seguidos já entrariam no bloqueio
            assert_eq!(p().validar(entrada, T0), Resultado::Incorreto, "{entrada:?}");
        }
    }

    #[test]
    fn o_codigo_do_intervalo_anterior_ainda_vale_para_quem_digita_devagar() {
        let p = p();
        let antigo = p.codigo_atual(T0);
        assert_eq!(p.validar(&antigo, T0 + 30), Resultado::Aceito);
    }

    #[test]
    fn dois_intervalos_depois_o_codigo_ja_expirou() {
        let p = p();
        let antigo = p.codigo_atual(T0);
        assert_eq!(p.validar(&antigo, T0 + 60), Resultado::Incorreto);
    }

    #[test]
    fn cada_codigo_so_pareia_uma_vez() {
        let p = p();
        let codigo = p.codigo_atual(T0);
        assert_eq!(p.validar(&codigo, T0), Resultado::Aceito);
        assert_eq!(p.validar(&codigo, T0 + 1), Resultado::Incorreto);
    }

    #[test]
    fn usar_o_codigo_atual_invalida_tambem_o_anterior_mas_o_proximo_funciona() {
        let p = p();
        let anterior = p.codigo_atual(T0 - 30);
        let atual = p.codigo_atual(T0);
        assert_eq!(p.validar(&atual, T0), Resultado::Aceito);
        assert_eq!(p.validar(&anterior, T0), Resultado::Incorreto);
        assert_eq!(p.validar(&p.codigo_atual(T0 + 30), T0 + 30), Resultado::Aceito);
    }

    #[test]
    fn cinco_erros_em_um_minuto_bloqueiam_ate_o_codigo_certo() {
        let p = p();
        let certo = p.codigo_atual(T0);
        let ruim = errado(&certo);
        for i in 0..5 {
            assert_eq!(p.validar(&ruim, T0 + i), Resultado::Incorreto);
        }
        match p.validar(&certo, T0 + 5) {
            Resultado::Bloqueado { segundos } => assert!((1..=60).contains(&segundos), "{segundos}"),
            outro => panic!("esperava bloqueio, veio {outro:?}"),
        }
    }

    #[test]
    fn o_bloqueio_passa_depois_de_um_minuto() {
        let p = p();
        let ruim = errado(&p.codigo_atual(T0));
        for i in 0..5 {
            p.validar(&ruim, T0 + i);
        }
        let depois = T0 + 61;
        assert_eq!(p.validar(&p.codigo_atual(depois), depois), Resultado::Aceito);
    }

    #[test]
    fn acertar_zera_a_contagem_de_erros() {
        let p = p();
        let ruim = errado(&p.codigo_atual(T0));
        for i in 0..4 {
            p.validar(&ruim, T0 + i);
        }
        assert_eq!(p.validar(&p.codigo_atual(T0 + 4), T0 + 4), Resultado::Aceito);
        // se a contagem não tivesse zerado, o próximo erro já bloquearia
        let ruim2 = errado(&p.codigo_atual(T0 + 35));
        for i in 0..4 {
            assert_eq!(p.validar(&ruim2, T0 + 35 + i), Resultado::Incorreto);
        }
    }

    #[test]
    fn erros_antigos_expiram_e_nao_se_acumulam() {
        let p = p();
        let ruim = errado(&p.codigo_atual(T0));
        for i in 0..4 {
            p.validar(&ruim, T0 + i);
        }
        for i in 0..4 {
            assert_eq!(p.validar(&ruim, T0 + 100 + i), Resultado::Incorreto);
        }
    }

    #[test]
    fn cada_servidor_tem_seu_proprio_segredo() {
        let a = Pareamento::novo().unwrap();
        let b = Pareamento::novo().unwrap();
        let iguais = (0..5).filter(|i| a.codigo_atual(T0 + i * 30) == b.codigo_atual(T0 + i * 30)).count();
        assert!(iguais < 5);
    }

    #[test]
    fn expoe_o_tempo_restante_do_codigo() {
        let p = p();
        assert_eq!(p.segundos_restantes(T0), crate::totp::segundos_restantes(T0));
    }
}
