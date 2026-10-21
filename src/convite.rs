//! O convite para o celular: o endereço com o token e o QR code para escanear no terminal.

use qrcode::{QrCode, render::unicode};

/// Endereço que o celular abre. O token vai na query, e a página o guarda e o tira da barra.
pub fn url_de_acesso(ip: &str, porta: u16, token: &str) -> String {
    format!("http://{ip}:{porta}/?t={token}")
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
    fn url_de_acesso_leva_o_token_na_query() {
        assert_eq!(
            url_de_acesso("192.168.68.103", 8080, "abc123"),
            "http://192.168.68.103:8080/?t=abc123"
        );
    }

    #[test]
    fn qr_code_vira_texto_com_varias_linhas_de_blocos() {
        let qr = qr_em_texto("http://192.168.68.103:8080/?t=abc123");
        assert!(qr.lines().count() > 10, "{qr}");
        assert!(qr.contains('█') || qr.contains('▀') || qr.contains('▄'));
    }

    #[test]
    fn texto_grande_demais_nao_derruba_o_programa() {
        assert_eq!(qr_em_texto(&"x".repeat(10_000)), "");
    }
}
