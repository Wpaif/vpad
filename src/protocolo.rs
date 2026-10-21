//! Protocolo binário celular -> PC:
//!   `[1, id, down]`     botão (0..=9 botões, 10..=13 direcional)
//!   `[2, id, lo, hi]`   eixo, i16 little-endian (0..=3 analógicos, 4..=5 gatilhos 0..=255)
//!   `[3, ..8 bytes]`    ping, devolvido igual ao celular
//!   `[4, indice]`       perfil: o celular pede a troca e o servidor informa o perfil ativo
//!   `[5, motivo]`       (servidor -> celular) acesso recusado: 1 = ocupado, 2 = não autorizado

use crate::perfil::PERFIS;

const MSG_BOTAO: u8 = 1;
const MSG_EIXO: u8 = 2;
const MSG_PING: u8 = 3;
const MSG_PERFIL: u8 = 4;
const MSG_RECUSA: u8 = 5;

const ULTIMO_BOTAO: u8 = 13;
const ULTIMO_EIXO: u8 = 5;
const PRIMEIRO_GATILHO: u8 = 4;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Evento {
    Botao { id: u8, pressionado: bool },
    Eixo { id: u8, valor: i16 },
    Ping,
    /// Pedido do celular para usar outro perfil (índice em `PERFIS`).
    TrocarPerfil(u8),
    /// Gerado pelo servidor quando o celular desconecta.
    SoltarTudo,
}

/// Por que o servidor recusou a conexão.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Recusa {
    /// outro aparelho já está controlando
    Ocupado = 1,
    /// token ausente ou errado
    NaoAutorizado = 2,
}

pub fn codificar_recusa(motivo: Recusa) -> [u8; 2] {
    [MSG_RECUSA, motivo as u8]
}

/// Mensagem que informa ao celular qual perfil está ativo.
pub fn codificar_perfil(indice: u8) -> [u8; 2] {
    [MSG_PERFIL, indice]
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
        [MSG_PERFIL, indice] if (indice as usize) < PERFIS.len() => Some(Evento::TrocarPerfil(indice)),
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

    #[test]
    fn decodifica_troca_de_perfil() {
        assert_eq!(decodificar(&[4, 0]), Some(Evento::TrocarPerfil(0)));
        assert_eq!(decodificar(&[4, 2]), Some(Evento::TrocarPerfil(2)));
        assert_eq!(decodificar(&[4, 3]), Some(Evento::TrocarPerfil(3))); // teclado
    }

    #[test]
    fn rejeita_perfil_fora_da_lista() {
        assert_eq!(decodificar(&[4, 4]), None);
        assert_eq!(decodificar(&[4]), None);
        assert_eq!(decodificar(&[4, 0, 0]), None);
    }

    #[test]
    fn codifica_o_perfil_atual_para_o_celular() {
        assert_eq!(codificar_perfil(2), [4, 2]);
    }

    #[test]
    fn codifica_os_motivos_de_recusa_para_o_celular() {
        assert_eq!(codificar_recusa(Recusa::Ocupado), [5, 1]);
        assert_eq!(codificar_recusa(Recusa::NaoAutorizado), [5, 2]);
    }
}
