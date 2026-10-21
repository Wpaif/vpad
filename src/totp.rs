//! Código de 6 dígitos que muda a cada 30 segundos (TOTP, RFC 6238, HMAC-SHA1): o mesmo
//! esquema dos aplicativos autenticadores, usado aqui para parear um celular novo.

use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;

pub const PASSO_SEGUNDOS: u64 = 30;

/// Número do intervalo de 30 s a que `segundos` (desde 1970) pertence.
pub fn passo(segundos: u64) -> u64 {
    segundos / PASSO_SEGUNDOS
}

/// Quanto falta para o código atual expirar (de 1 a 30).
pub fn segundos_restantes(segundos: u64) -> u64 {
    PASSO_SEGUNDOS - segundos % PASSO_SEGUNDOS
}

/// Código de 6 dígitos (com zeros à esquerda) do intervalo `passo`.
pub fn codigo(segredo: &[u8], passo: u64) -> String {
    let mut mac = <Hmac<Sha1> as KeyInit>::new_from_slice(segredo).expect("HMAC aceita chave de qualquer tamanho");
    mac.update(&passo.to_be_bytes());
    let hash = mac.finalize().into_bytes();
    let desvio = (hash[19] & 0x0f) as usize;
    let numero = u32::from_be_bytes([hash[desvio] & 0x7f, hash[desvio + 1], hash[desvio + 2], hash[desvio + 3]]);
    format!("{:06}", numero % 1_000_000)
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Vetores do RFC 6238 (HMAC-SHA1, segredo "12345678901234567890"). O RFC lista 8 dígitos;
    /// os 6 dígitos são os 6 finais do mesmo número.
    const SEGREDO: &[u8] = b"12345678901234567890";

    #[test]
    fn bate_com_os_vetores_do_rfc_6238() {
        let casos = [
            (59, "287082"),
            (1_111_111_109, "081804"),
            (1_111_111_111, "050471"),
            (1_234_567_890, "005924"),
            (2_000_000_000, "279037"),
            (20_000_000_000, "353130"),
        ];
        for (segundos, esperado) in casos {
            assert_eq!(codigo(SEGREDO, passo(segundos)), esperado, "t = {segundos}");
        }
    }

    #[test]
    fn o_codigo_sempre_tem_seis_digitos_inclusive_com_zeros_na_frente() {
        for p in 0..2000 {
            let c = codigo(SEGREDO, p);
            assert_eq!(c.len(), 6, "{c}");
            assert!(c.chars().all(|d| d.is_ascii_digit()), "{c}");
        }
        assert_eq!(codigo(SEGREDO, passo(1_234_567_890)), "005924");
    }

    #[test]
    fn segredos_diferentes_dao_codigos_diferentes() {
        let iguais = (0..50).filter(|p| codigo(b"segredo-um-bem-longo", *p) == codigo(b"segredo-dois-bem-longo", *p)).count();
        assert!(iguais < 3, "{iguais} coincidências em 50 passos");
    }

    #[test]
    fn o_codigo_muda_a_cada_30_segundos() {
        assert_eq!(passo(0), passo(29));
        assert_ne!(passo(29), passo(30));
        assert_eq!(passo(30), passo(59));
        assert_eq!(PASSO_SEGUNDOS, 30);
    }

    #[test]
    fn segundos_restantes_ate_o_proximo_codigo() {
        assert_eq!(segundos_restantes(0), 30);
        assert_eq!(segundos_restantes(1), 29);
        assert_eq!(segundos_restantes(29), 1);
        assert_eq!(segundos_restantes(30), 30);
    }
}
