//! O convite para o celular: o endereço da página e o QR code para escanear no terminal.

use qrcode::{QrCode, render::unicode};

/// Endereço que o celular abre. Não leva segredo: o celular se identifica digitando o código
/// de pareamento.
pub fn url_de_acesso(ip: &str, porta: u16) -> String {
    format!("http://{ip}:{porta}/")
}

/// QR code em blocos Unicode, para mostrar no terminal. Vazio se o texto não couber num QR.
pub fn qr_em_texto(texto: &str) -> String {
    QrCode::new(texto)
        .map(|codigo| codigo.render::<unicode::Dense1x2>().quiet_zone(true).build())
        .unwrap_or_default()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn a_url_do_convite_nao_leva_segredo_nenhum() {
        // o acesso vem do código de pareamento digitado no celular, não da URL
        assert_eq!(url_de_acesso("192.168.68.103", 8080), "http://192.168.68.103:8080/");
    }

    #[test]
    fn qr_code_vira_texto_com_varias_linhas_de_blocos() {
        let qr = qr_em_texto("http://192.168.68.103:8080/");
        assert!(qr.lines().count() > 10, "{qr}");
        assert!(qr.contains('█') || qr.contains('▀') || qr.contains('▄'));
    }

    #[test]
    fn texto_grande_demais_nao_derruba_o_programa() {
        assert_eq!(qr_em_texto(&"x".repeat(10_000)), "");
    }
}
