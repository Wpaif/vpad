//! Avisos de segurança que o servidor mostra no terminal do PC. Tentativas de pareamento
//! erradas não podem passar em silêncio: quem está diante do PC precisa poder notar.

use std::{fmt, net::IpAddr, sync::Arc};

use crate::{mensagens::txt, tr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alerta {
    /// código de pareamento errado; `erros` é a contagem na janela de 1 minuto
    CodigoErrado { origem: Option<IpAddr>, erros: usize },
    /// o pareamento foi bloqueado por excesso de erros
    Bloqueado { origem: Option<IpAddr>, segundos: u64 },
    /// alguém tentou parear com o pareamento fechado
    PareamentoFechado { origem: Option<IpAddr> },
    /// um aparelho novo foi pareado (depois da aprovação no PC)
    Pareado { nome: String, id: String, origem: Option<IpAddr> },
    /// o pedido de pareamento foi negado no PC
    PareamentoNegado { origem: Option<IpAddr> },
    /// ninguém aprovou o pedido de pareamento a tempo
    PareamentoExpirou { origem: Option<IpAddr> },
}

fn origem(o: &Option<IpAddr>) -> String {
    o.map_or_else(|| txt("origem desconhecida", "unknown origin").to_string(), |ip| tr!("de {ip}", "from {ip}"))
}

impl fmt::Display for Alerta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Alerta::CodigoErrado { origem: o, erros } => write!(
                f,
                "{}",
                tr!(
                    "⚠ código de pareamento errado ({}): {erros} erro(s) no último minuto",
                    "⚠ wrong pairing code ({}): {erros} error(s) in the last minute",
                    origem(o)
                )
            ),
            Alerta::Bloqueado { origem: o, segundos } => write!(
                f,
                "{}",
                tr!(
                    "⚠ pareamento BLOQUEADO por {segundos} s após erros seguidos (último {})",
                    "⚠ pairing BLOCKED for {segundos} s after repeated errors (last {})",
                    origem(o)
                )
            ),
            Alerta::PareamentoFechado { origem: o } => write!(
                f,
                "{}",
                tr!(
                    "⚠ tentativa de pareamento ({}), mas o pareamento está fechado; pressione Enter para abrir",
                    "⚠ pairing attempt ({}), but pairing is closed; press Enter to open it",
                    origem(o)
                )
            ),
            Alerta::Pareado { nome, id, origem: o } => {
                write!(
                    f,
                    "{}",
                    tr!("✓ dispositivo pareado: '{nome}' (id {id}, {})", "✓ device paired: '{nome}' (id {id}, {})", origem(o))
                )
            }
            Alerta::PareamentoNegado { origem: o } => {
                write!(f, "{}", tr!("pedido de pareamento negado ({})", "pairing request denied ({})", origem(o)))
            }
            Alerta::PareamentoExpirou { origem: o } => {
                write!(
                    f,
                    "{}",
                    tr!("pedido de pareamento expirou sem resposta ({})", "pairing request expired without an answer ({})", origem(o))
                )
            }
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
    fn os_resultados_do_pedido_dizem_o_que_aconteceu_e_de_onde() {
        let pareado = Alerta::Pareado { nome: "iPhone".into(), id: "ab12cd34".into(), origem: ip("10.0.0.9") }.to_string();
        assert!(pareado.contains("iPhone") && pareado.contains("ab12cd34") && pareado.contains("10.0.0.9"), "{pareado}");
        let negado = Alerta::PareamentoNegado { origem: ip("10.0.0.9") }.to_string();
        assert!(negado.contains("negado") && negado.contains("10.0.0.9"), "{negado}");
        let expirou = Alerta::PareamentoExpirou { origem: ip("10.0.0.9") }.to_string();
        assert!(expirou.contains("expirou") && expirou.contains("10.0.0.9"), "{expirou}");
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

#[cfg(test)]
mod ingles {
    use super::*;
    use crate::{idiomas::Idioma, mensagens::com_idioma};

    fn ip() -> Option<IpAddr> {
        Some("192.168.68.50".parse().unwrap())
    }

    fn en(a: Alerta) -> String {
        com_idioma(Idioma::En, || a.to_string())
    }

    #[test]
    fn cada_alerta_em_ingles() {
        assert_eq!(
            en(Alerta::CodigoErrado { origem: ip(), erros: 3 }),
            "⚠ wrong pairing code (from 192.168.68.50): 3 error(s) in the last minute"
        );
        assert_eq!(
            en(Alerta::Bloqueado { origem: ip(), segundos: 60 }),
            "⚠ pairing BLOCKED for 60 s after repeated errors (last from 192.168.68.50)"
        );
        assert_eq!(
            en(Alerta::PareamentoFechado { origem: ip() }),
            "⚠ pairing attempt (from 192.168.68.50), but pairing is closed; press Enter to open it"
        );
        assert_eq!(
            en(Alerta::Pareado { nome: "iPhone".into(), id: "abcd1234".into(), origem: ip() }),
            "✓ device paired: 'iPhone' (id abcd1234, from 192.168.68.50)"
        );
        assert_eq!(en(Alerta::PareamentoNegado { origem: ip() }), "pairing request denied (from 192.168.68.50)");
        assert_eq!(
            en(Alerta::PareamentoExpirou { origem: ip() }),
            "pairing request expired without an answer (from 192.168.68.50)"
        );
    }

    #[test]
    fn origem_desconhecida_em_ingles() {
        assert_eq!(en(Alerta::PareamentoNegado { origem: None }), "pairing request denied (unknown origin)");
    }

    #[test]
    fn em_portugues_nada_mudou() {
        assert_eq!(
            Alerta::PareamentoNegado { origem: ip() }.to_string(),
            "pedido de pareamento negado (de 192.168.68.50)"
        );
        assert_eq!(
            Alerta::CodigoErrado { origem: None, erros: 1 }.to_string(),
            "⚠ código de pareamento errado (origem desconhecida): 1 erro(s) no último minuto"
        );
    }
}
