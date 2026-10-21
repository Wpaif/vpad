//! Avisos de segurança que o servidor mostra no terminal do PC. Tentativas de pareamento
//! erradas não podem passar em silêncio: quem está diante do PC precisa poder notar.

use std::{fmt, net::IpAddr, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alerta {
    /// código de pareamento errado; `erros` é a contagem na janela de 1 minuto
    CodigoErrado { origem: Option<IpAddr>, erros: usize },
    /// o pareamento foi bloqueado por excesso de erros
    Bloqueado { origem: Option<IpAddr>, segundos: u64 },
    /// alguém tentou parear com o pareamento fechado
    PareamentoFechado { origem: Option<IpAddr> },
}

fn origem(o: &Option<IpAddr>) -> String {
    o.map_or_else(|| "origem desconhecida".to_string(), |ip| format!("de {ip}"))
}

impl fmt::Display for Alerta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Alerta::CodigoErrado { origem: o, erros } => write!(
                f,
                "⚠ código de pareamento errado ({}): {erros} erro(s) no último minuto",
                origem(o)
            ),
            Alerta::Bloqueado { origem: o, segundos } => write!(
                f,
                "⚠ pareamento BLOQUEADO por {segundos} s após erros seguidos (último {})",
                origem(o)
            ),
            Alerta::PareamentoFechado { origem: o } => write!(
                f,
                "⚠ tentativa de pareamento ({}), mas o pareamento está fechado; pressione Enter para abrir",
                origem(o)
            ),
        }
    }
}

/// Para onde os alertas vão (o terminal, ou uma lista nos testes).
pub type Alertas = Arc<dyn Fn(Alerta) + Send + Sync>;

/// Destino que descarta tudo.
pub fn ignorar() -> Alertas {
    Arc::new(|_| {})
}

#[cfg(test)]
mod testes {
    use super::*;

    fn ip(s: &str) -> Option<IpAddr> {
        Some(s.parse().unwrap())
    }

    #[test]
    fn codigo_errado_mostra_a_origem_e_a_contagem() {
        let texto = Alerta::CodigoErrado { origem: ip("192.168.68.77"), erros: 3 }.to_string();
        assert!(texto.contains("192.168.68.77") && texto.contains('3'), "{texto}");
        assert!(texto.contains("errado"), "{texto}");
    }

    #[test]
    fn bloqueio_mostra_origem_e_duracao() {
        let texto = Alerta::Bloqueado { origem: ip("10.0.0.9"), segundos: 300 }.to_string();
        assert!(texto.contains("10.0.0.9") && texto.contains("300"), "{texto}");
        assert!(texto.to_lowercase().contains("bloque"), "{texto}");
    }

    #[test]
    fn tentativa_com_pareamento_fechado_diz_como_abrir() {
        let texto = Alerta::PareamentoFechado { origem: ip("10.0.0.9") }.to_string();
        assert!(texto.contains("10.0.0.9") && texto.to_lowercase().contains("fechado"), "{texto}");
        assert!(texto.contains("Enter"), "{texto}");
    }

    #[test]
    fn origem_desconhecida_tambem_tem_texto() {
        for a in [
            Alerta::CodigoErrado { origem: None, erros: 1 },
            Alerta::Bloqueado { origem: None, segundos: 60 },
            Alerta::PareamentoFechado { origem: None },
        ] {
            assert!(a.to_string().contains("origem desconhecida"), "{a}");
        }
    }

    #[test]
    fn o_destino_padrao_ignora_tudo_sem_quebrar() {
        ignorar()(Alerta::PareamentoFechado { origem: None });
    }
}
