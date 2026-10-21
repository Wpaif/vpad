//! Protocolo binário celular -> PC:
//!   `[1, id, down]`     botão (0..=9 botões, 10..=13 direcional)
//!   `[2, id, lo, hi]`   eixo, i16 little-endian (0..=3 analógicos, 4..=5 gatilhos 0..=255)
//!   `[3, ..8 bytes]`    ping, devolvido igual ao celular

const MSG_BOTAO: u8 = 1;
const MSG_EIXO: u8 = 2;
const MSG_PING: u8 = 3;

const ULTIMO_BOTAO: u8 = 13;
const ULTIMO_EIXO: u8 = 5;
const PRIMEIRO_GATILHO: u8 = 4;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Evento {
    Botao { id: u8, pressionado: bool },
    Eixo { id: u8, valor: i16 },
    Ping,
    /// Gerado pelo servidor quando o celular desconecta.
    SoltarTudo,
}

pub fn decodificar(dados: &[u8]) -> Option<Evento> {
    match *dados {
        [MSG_BOTAO, id, estado] if id <= ULTIMO_BOTAO => Some(Evento::Botao {
            id,
            pressionado: estado != 0,
        }),
        [MSG_EIXO, id, lo, hi] if id <= ULTIMO_EIXO => {
            let valor = i16::from_le_bytes([lo, hi]);
            let valor = if id >= PRIMEIRO_GATILHO {
                valor.clamp(0, 255)
            } else {
                valor
            };
            Some(Evento::Eixo { id, valor })
        }
        [MSG_PING, ..] if dados.len() == 9 => Some(Evento::Ping),
        _ => None,
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn decodifica_botao() {
        assert_eq!(
            decodificar(&[1, 0, 1]),
            Some(Evento::Botao { id: 0, pressionado: true })
        );
        assert_eq!(
            decodificar(&[1, 13, 0]),
            Some(Evento::Botao { id: 13, pressionado: false })
        );
    }

    #[test]
    fn rejeita_botao_fora_da_faixa() {
        assert_eq!(decodificar(&[1, 14, 1]), None);
    }

    #[test]
    fn decodifica_eixo_i16_little_endian() {
        assert_eq!(
            decodificar(&[2, 0, 0x34, 0x12]),
            Some(Evento::Eixo { id: 0, valor: 0x1234 })
        );
        assert_eq!(
            decodificar(&[2, 3, 0xff, 0xff]),
            Some(Evento::Eixo { id: 3, valor: -1 })
        );
    }

    #[test]
    fn gatilhos_ficam_entre_0_e_255() {
        assert_eq!(
            decodificar(&[2, 4, 0xff, 0x7f]),
            Some(Evento::Eixo { id: 4, valor: 255 })
        );
        assert_eq!(
            decodificar(&[2, 5, 0x00, 0x80]),
            Some(Evento::Eixo { id: 5, valor: 0 })
        );
    }

    #[test]
    fn rejeita_eixo_fora_da_faixa() {
        assert_eq!(decodificar(&[2, 6, 0, 0]), None);
    }

    #[test]
    fn ping_so_com_nove_bytes() {
        assert_eq!(decodificar(&[3, 0, 0, 0, 0, 0, 0, 0, 0]), Some(Evento::Ping));
        assert_eq!(decodificar(&[3, 0, 0]), None);
    }

    #[test]
    fn mensagens_malformadas_viram_none() {
        assert_eq!(decodificar(&[]), None);
        assert_eq!(decodificar(&[9, 1, 1]), None);
        assert_eq!(decodificar(&[1, 0]), None);
        assert_eq!(decodificar(&[2, 0, 1]), None);
    }
}
