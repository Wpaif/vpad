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
pub const MAX_ERROS: usize = 5;
const JANELA_DE_ERROS: u64 = 60;
/// Duração (segundos) de cada bloqueio seguido: 1 min, 5 min e, daí em diante, 30 min.
const ESPERAS: [u64; 3] = [60, 300, 1800];
/// Sem novos bloqueios por tanto tempo, a escada volta ao primeiro degrau.
const ESCADA_ESFRIA_APOS: u64 = 6 * 3600;

#[derive(Debug, PartialEq, Eq)]
pub enum Resultado {
    Aceito,
    Incorreto,
    /// o pareamento não está aberto: nem o código certo vale
    Fechado,
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
    /// o pareamento só aceita códigos enquanto estiver aberto (por ordem de quem está no PC)
    aberto_ate: Option<u64>,
    bloqueado_ate: Option<u64>,
    /// degrau atual da escada de bloqueios e quando foi o último bloqueio
    degrau: usize,
    ultimo_bloqueio: Option<u64>,
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

    /// Segundos até o código mostrado agora mudar.
    pub fn segundos_restantes_do_codigo(&self, agora: u64) -> u64 {
        totp::segundos_restantes(agora)
    }

    /// Abre a janela de pareamento por `duracao` segundos (abrir de novo renova o prazo).
    pub fn abrir(&self, agora: u64, duracao: u64) {
        self.estado.lock().unwrap().aberto_ate = Some(agora + duracao);
    }

    pub fn fechar(&self) {
        self.estado.lock().unwrap().aberto_ate = None;
    }

    pub fn aberto(&self, agora: u64) -> bool {
        self.estado.lock().unwrap().aberto_ate.is_some_and(|fim| agora < fim)
    }

    pub fn segundos_abertos_restantes(&self, agora: u64) -> u64 {
        self.estado.lock().unwrap().aberto_ate.map_or(0, |fim| fim.saturating_sub(agora))
    }

    /// Segundos até o bloqueio atual acabar, se houver um.
    pub fn bloqueio_restante(&self, agora: u64) -> Option<u64> {
        let estado = self.estado.lock().unwrap();
        estado.bloqueado_ate.filter(|fim| agora < *fim).map(|fim| fim - agora)
    }

    /// Quantos erros contam hoje na janela de 1 minuto (para os alertas).
    pub fn erros_recentes(&self, agora: u64) -> usize {
        let estado = self.estado.lock().unwrap();
        estado.erros.iter().filter(|t| agora.saturating_sub(**t) < JANELA_DE_ERROS).count()
    }

    /// Confere o código digitado. Vale o do intervalo atual e o do anterior (quem digita
    /// devagar não perde o código por um segundo), uma única vez cada um. Só funciona com a
    /// janela aberta; erros seguidos bloqueiam por períodos cada vez maiores.
    pub fn validar(&self, informado: &str, agora: u64) -> Resultado {
        let mut estado = self.estado.lock().unwrap();
        if !estado.aberto_ate.is_some_and(|fim| agora < fim) {
            return Resultado::Fechado;
        }
        if let Some(fim) = estado.bloqueado_ate {
            if agora < fim {
                return Resultado::Bloqueado { segundos: fim - agora };
            }
            estado.bloqueado_ate = None;
        }
        if estado.degrau > 0 && estado.ultimo_bloqueio.is_some_and(|t| agora.saturating_sub(t) >= ESCADA_ESFRIA_APOS) {
            estado.degrau = 0;
        }
        while estado.erros.front().is_some_and(|t| agora.saturating_sub(*t) >= JANELA_DE_ERROS) {
            estado.erros.pop_front();
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
                estado.degrau = 0;
                estado.ultimo_bloqueio = None;
                Resultado::Aceito
            }
            None => {
                estado.erros.push_back(agora);
                if estado.erros.len() >= MAX_ERROS {
                    let duracao = ESPERAS[estado.degrau.min(ESPERAS.len() - 1)];
                    estado.bloqueado_ate = Some(agora + duracao);
                    estado.degrau += 1;
                    estado.ultimo_bloqueio = Some(agora);
                    estado.erros.clear();
                }
                Resultado::Incorreto
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const T0: u64 = 1_700_000_010; // início de um intervalo de 30 s + 10 s

    /// Pareamento com a janela aberta por muito tempo, para os testes do código em si.
    fn p() -> Pareamento {
        let p = fechado();
        p.abrir(T0, 100_000);
        p
    }

    fn fechado() -> Pareamento {
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
        let depois = T0 + 65; // o bloqueio começa no 5º erro (T0 + 4) e dura 60 s
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
        assert_eq!(p.segundos_restantes_do_codigo(T0), crate::totp::segundos_restantes(T0));
    }

    // ---------- janela de pareamento (fechada por padrão) ----------

    #[test]
    fn comeca_fechado_e_nao_aceita_nem_o_codigo_certo() {
        let p = fechado();
        assert!(!p.aberto(T0));
        assert_eq!(p.validar(&p.codigo_atual(T0), T0), Resultado::Fechado);
    }

    #[test]
    fn abrir_vale_pela_duracao_pedida_e_depois_fecha_sozinho() {
        let p = fechado();
        p.abrir(T0, 120);
        assert!(p.aberto(T0) && p.aberto(T0 + 119));
        assert!(!p.aberto(T0 + 120));
        assert_eq!(p.validar(&p.codigo_atual(T0 + 120), T0 + 120), Resultado::Fechado);
    }

    #[test]
    fn fechar_fecha_na_hora() {
        let p = fechado();
        p.abrir(T0, 120);
        p.fechar();
        assert_eq!(p.validar(&p.codigo_atual(T0 + 1), T0 + 1), Resultado::Fechado);
    }

    #[test]
    fn abrir_de_novo_renova_o_prazo() {
        let p = fechado();
        p.abrir(T0, 120);
        p.abrir(T0 + 100, 120);
        assert!(p.aberto(T0 + 200));
        assert!(!p.aberto(T0 + 220));
    }

    #[test]
    fn expoe_quanto_falta_para_a_janela_fechar() {
        let p = fechado();
        assert_eq!(p.segundos_abertos_restantes(T0), 0);
        p.abrir(T0, 120);
        assert_eq!(p.segundos_abertos_restantes(T0 + 20), 100);
        assert_eq!(p.segundos_abertos_restantes(T0 + 500), 0);
    }

    #[test]
    fn tentativas_com_a_janela_fechada_nao_contam_como_erro_nem_bloqueiam() {
        let p = fechado();
        for i in 0..20 {
            assert_eq!(p.validar("000000", T0 + i), Resultado::Fechado);
        }
        p.abrir(T0 + 30, 120);
        assert_eq!(p.validar(&p.codigo_atual(T0 + 31), T0 + 31), Resultado::Aceito);
    }

    // ---------- escada de bloqueios ----------

    /// Cinco erros seguidos a partir de `inicio`; devolve o instante do último.
    fn cinco_erros(p: &Pareamento, inicio: u64) -> u64 {
        let ruim = errado(&p.codigo_atual(inicio));
        for i in 0..5 {
            p.validar(&ruim, inicio + i);
        }
        inicio + 4
    }

    fn espera(p: &Pareamento, quando: u64) -> u64 {
        match p.validar(&p.codigo_atual(quando), quando) {
            Resultado::Bloqueado { segundos } => segundos,
            outro => panic!("esperava bloqueio, veio {outro:?}"),
        }
    }

    #[test]
    fn cada_bloqueio_seguido_dura_mais_ate_o_teto() {
        let p = p();
        let fim = cinco_erros(&p, T0);
        assert_eq!(espera(&p, fim), 60);

        let t2 = fim + 61;
        let fim2 = cinco_erros(&p, t2);
        assert_eq!(espera(&p, fim2), 300);

        let t3 = fim2 + 301;
        let fim3 = cinco_erros(&p, t3);
        assert_eq!(espera(&p, fim3), 1800);

        let t4 = fim3 + 1801;
        let fim4 = cinco_erros(&p, t4);
        assert_eq!(espera(&p, fim4), 1800, "o teto é de 30 minutos");
    }

    #[test]
    fn tentar_durante_o_bloqueio_nao_o_prolonga() {
        let p = p();
        let fim = cinco_erros(&p, T0);
        assert_eq!(espera(&p, fim + 10), 50);
        for i in 11..30 {
            p.validar("000000", fim + i);
        }
        assert_eq!(espera(&p, fim + 30), 30, "o prazo continua correndo normalmente");
    }

    #[test]
    fn acertar_o_codigo_zera_a_escada() {
        let p = p();
        let fim = cinco_erros(&p, T0);
        let depois = fim + 61;
        assert_eq!(p.validar(&p.codigo_atual(depois), depois), Resultado::Aceito);
        p.abrir(depois, 100_000);
        let fim2 = cinco_erros(&p, depois + 40);
        assert_eq!(espera(&p, fim2), 60, "voltou ao primeiro degrau");
    }

    #[test]
    fn a_escada_esfria_depois_de_seis_horas_sem_bloqueios() {
        let p = p();
        let fim = cinco_erros(&p, T0);
        let fim2 = cinco_erros(&p, fim + 61);
        assert_eq!(espera(&p, fim2), 300);

        let muito_depois = fim2 + 6 * 3600 + 301;
        let fim3 = cinco_erros(&p, muito_depois);
        assert_eq!(espera(&p, fim3), 60);
    }

    #[test]
    fn informa_quantos_erros_ha_na_janela_para_os_alertas() {
        let p = p();
        assert_eq!(p.erros_recentes(T0), 0);
        let ruim = errado(&p.codigo_atual(T0));
        p.validar(&ruim, T0);
        p.validar(&ruim, T0 + 1);
        assert_eq!(p.erros_recentes(T0 + 2), 2);
        assert_eq!(p.erros_recentes(T0 + 100), 0, "erros antigos saem da conta");
    }

    #[test]
    fn informa_quanto_falta_para_o_bloqueio_acabar() {
        let p = p();
        assert_eq!(p.bloqueio_restante(T0), None);
        let fim = cinco_erros(&p, T0);
        assert_eq!(p.bloqueio_restante(fim), Some(60));
        assert_eq!(p.bloqueio_restante(fim + 59), Some(1));
        assert_eq!(p.bloqueio_restante(fim + 60), None);
    }
}
