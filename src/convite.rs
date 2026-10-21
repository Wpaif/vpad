//! O convite para o celular: o endereço da página e o QR code para escanear no terminal.

use qrcode::{QrCode, render::unicode};

/// Endereço do controle, sempre HTTPS. Não leva segredo: o celular se identifica digitando o
/// código de pareamento.
pub fn url_do_controle(ip: &str, porta: u16) -> String {
    format!("https://{ip}:{porta}/")
}

/// Endereço da página que entrega o certificado, em HTTP simples.
pub fn url_de_configuracao(ip: &str, porta: u16) -> String {
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
    fn a_url_do_controle_e_https_e_nao_leva_segredo_nenhum() {
        // o acesso vem do código de pareamento digitado no celular, não da URL
        assert_eq!(url_do_controle("192.168.68.103", 8443), "https://192.168.68.103:8443/");
    }

    #[test]
    fn a_url_da_configuracao_do_certificado_e_http() {
        // é por HTTP de propósito: o celular ainda não confia no certificado quando a abre
        assert_eq!(url_de_configuracao("192.168.68.103", 8080), "http://192.168.68.103:8080/");
    }

    #[test]
    fn qr_code_vira_texto_com_varias_linhas_de_blocos() {
        let qr = qr_em_texto("https://192.168.68.103:8443/");
        assert!(qr.lines().count() > 10, "{qr}");
        assert!(qr.contains('█') || qr.contains('▀') || qr.contains('▄'));
    }

    #[test]
    fn texto_grande_demais_nao_derruba_o_programa() {
        assert_eq!(qr_em_texto(&"x".repeat(10_000)), "");
    }
}
