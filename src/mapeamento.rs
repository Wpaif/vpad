//! Traduz os eventos do protocolo em eventos de gamepad do Linux (estilo Xbox 360),
//! sem tocar no sistema: quem escreve no uinput é o módulo `dispositivo`.

use evdev::{AbsoluteAxisCode as Abs, EventType, KeyCode};

use crate::protocolo::Evento;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct EventoLinux {
    pub tipo: u16,
    pub codigo: u16,
    pub valor: i32,
}

impl EventoLinux {
    fn tecla(codigo: KeyCode, valor: i32) -> Self {
        Self { tipo: EventType::KEY.0, codigo: codigo.0, valor }
    }

    fn eixo(codigo: Abs, valor: i32) -> Self {
        Self { tipo: EventType::ABSOLUTE.0, codigo: codigo.0, valor }
    }
}

pub const BOTOES: [KeyCode; 10] = [
    KeyCode::BTN_SOUTH,  // 0 A
    KeyCode::BTN_EAST,   // 1 B
    KeyCode::BTN_WEST,   // 2 X
    KeyCode::BTN_NORTH,  // 3 Y
    KeyCode::BTN_TL,     // 4 LB
    KeyCode::BTN_TR,     // 5 RB
    KeyCode::BTN_SELECT, // 6 Select
    KeyCode::BTN_START,  // 7 Start
    KeyCode::BTN_THUMBL, // 8 L3
    KeyCode::BTN_THUMBR, // 9 R3
];

pub const EIXOS: [Abs; 6] = [
    Abs::ABS_X,  // 0 analógico esquerdo X
    Abs::ABS_Y,  // 1 analógico esquerdo Y
    Abs::ABS_RX, // 2 analógico direito X
    Abs::ABS_RY, // 3 analógico direito Y
    Abs::ABS_Z,  // 4 gatilho esquerdo
    Abs::ABS_RZ, // 5 gatilho direito
];

const CIMA: u8 = 10;
const BAIXO: u8 = 11;
const ESQUERDA: u8 = 12;
const DIREITA: u8 = 13;

/// Guarda quais direções do direcional estão pressionadas: o hat do Linux é um
/// eixo só por dimensão, então esquerda+direita precisam virar 0.
#[derive(Default)]
pub struct Tradutor {
    cima: bool,
    baixo: bool,
    esquerda: bool,
    direita: bool,
}

impl Tradutor {
    pub fn traduzir(&mut self, evento: Evento) -> Vec<EventoLinux> {
        match evento {
            Evento::Botao { id, pressionado } => self.botao(id, pressionado),
            Evento::Eixo { id, valor } => EIXOS
                .get(id as usize)
                .map(|a| vec![EventoLinux::eixo(*a, valor as i32)])
                .unwrap_or_default(),
            Evento::Ping => vec![],
            Evento::SoltarTudo => self.soltar_tudo(),
        }
    }

    fn botao(&mut self, id: u8, pressionado: bool) -> Vec<EventoLinux> {
        match id {
            CIMA => self.cima = pressionado,
            BAIXO => self.baixo = pressionado,
            ESQUERDA => self.esquerda = pressionado,
            DIREITA => self.direita = pressionado,
            _ => {
                return BOTOES
                    .get(id as usize)
                    .map(|k| vec![EventoLinux::tecla(*k, pressionado as i32)])
                    .unwrap_or_default();
            }
        }
        let hat = if matches!(id, ESQUERDA | DIREITA) {
            EventoLinux::eixo(Abs::ABS_HAT0X, self.direita as i32 - self.esquerda as i32)
        } else {
            EventoLinux::eixo(Abs::ABS_HAT0Y, self.baixo as i32 - self.cima as i32)
        };
        vec![hat]
    }

    fn soltar_tudo(&mut self) -> Vec<EventoLinux> {
        *self = Self::default();
        let teclas = BOTOES.iter().map(|k| EventoLinux::tecla(*k, 0));
        let eixos = EIXOS.iter().map(|a| EventoLinux::eixo(*a, 0));
        let hats = [Abs::ABS_HAT0X, Abs::ABS_HAT0Y].map(|a| EventoLinux::eixo(a, 0));
        teclas.chain(eixos).chain(hats).collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::protocolo::Evento;
    use evdev::{AbsoluteAxisCode as Abs, EventType, KeyCode};

    fn tecla(k: KeyCode, v: i32) -> EventoLinux {
        EventoLinux { tipo: EventType::KEY.0, codigo: k.0, valor: v }
    }
    fn eixo(a: Abs, v: i32) -> EventoLinux {
        EventoLinux { tipo: EventType::ABSOLUTE.0, codigo: a.0, valor: v }
    }
    fn botao(id: u8, pressionado: bool) -> Evento {
        Evento::Botao { id, pressionado }
    }

    #[test]
    fn botoes_de_face_ombros_e_menu() {
        let tabela = [
            (0, KeyCode::BTN_SOUTH),
            (1, KeyCode::BTN_EAST),
            (2, KeyCode::BTN_WEST),
            (3, KeyCode::BTN_NORTH),
            (4, KeyCode::BTN_TL),
            (5, KeyCode::BTN_TR),
            (6, KeyCode::BTN_SELECT),
            (7, KeyCode::BTN_START),
            (8, KeyCode::BTN_THUMBL),
            (9, KeyCode::BTN_THUMBR),
        ];
        for (id, codigo) in tabela {
            let mut t = Tradutor::default();
            assert_eq!(t.traduzir(botao(id, true)), vec![tecla(codigo, 1)], "id {id}");
            assert_eq!(t.traduzir(botao(id, false)), vec![tecla(codigo, 0)], "id {id}");
        }
    }

    #[test]
    fn direcional_vira_hat() {
        let mut t = Tradutor::default();
        assert_eq!(t.traduzir(botao(12, true)), vec![eixo(Abs::ABS_HAT0X, -1)]); // esquerda
        assert_eq!(t.traduzir(botao(12, false)), vec![eixo(Abs::ABS_HAT0X, 0)]);
        assert_eq!(t.traduzir(botao(13, true)), vec![eixo(Abs::ABS_HAT0X, 1)]); // direita
        assert_eq!(t.traduzir(botao(13, false)), vec![eixo(Abs::ABS_HAT0X, 0)]);
        assert_eq!(t.traduzir(botao(10, true)), vec![eixo(Abs::ABS_HAT0Y, -1)]); // cima
        assert_eq!(t.traduzir(botao(10, false)), vec![eixo(Abs::ABS_HAT0Y, 0)]);
        assert_eq!(t.traduzir(botao(11, true)), vec![eixo(Abs::ABS_HAT0Y, 1)]); // baixo
        assert_eq!(t.traduzir(botao(11, false)), vec![eixo(Abs::ABS_HAT0Y, 0)]);
    }

    #[test]
    fn direcional_combina_direcoes_opostas_e_diagonais() {
        let mut t = Tradutor::default();
        t.traduzir(botao(12, true));
        // esquerda + direita juntas se anulam
        assert_eq!(t.traduzir(botao(13, true)), vec![eixo(Abs::ABS_HAT0X, 0)]);
        // soltar a esquerda deixa só a direita
        assert_eq!(t.traduzir(botao(12, false)), vec![eixo(Abs::ABS_HAT0X, 1)]);
        // diagonal: cima não mexe no eixo X
        assert_eq!(t.traduzir(botao(10, true)), vec![eixo(Abs::ABS_HAT0Y, -1)]);
    }

    #[test]
    fn analogicos_e_gatilhos() {
        let mut t = Tradutor::default();
        let casos = [
            (0, Abs::ABS_X),
            (1, Abs::ABS_Y),
            (2, Abs::ABS_RX),
            (3, Abs::ABS_RY),
            (4, Abs::ABS_Z),
            (5, Abs::ABS_RZ),
        ];
        for (id, abs) in casos {
            assert_eq!(
                t.traduzir(Evento::Eixo { id, valor: 123 }),
                vec![eixo(abs, 123)],
                "id {id}"
            );
        }
    }

    #[test]
    fn ping_nao_gera_evento() {
        assert!(Tradutor::default().traduzir(Evento::Ping).is_empty());
    }

    #[test]
    fn soltar_tudo_zera_botoes_eixos_e_hats() {
        let eventos = Tradutor::default().traduzir(Evento::SoltarTudo);
        for k in [
            KeyCode::BTN_SOUTH, KeyCode::BTN_EAST, KeyCode::BTN_WEST, KeyCode::BTN_NORTH,
            KeyCode::BTN_TL, KeyCode::BTN_TR, KeyCode::BTN_SELECT, KeyCode::BTN_START,
            KeyCode::BTN_THUMBL, KeyCode::BTN_THUMBR,
        ] {
            assert!(eventos.contains(&tecla(k, 0)), "{k:?}");
        }
        for a in [
            Abs::ABS_X, Abs::ABS_Y, Abs::ABS_RX, Abs::ABS_RY, Abs::ABS_Z, Abs::ABS_RZ,
            Abs::ABS_HAT0X, Abs::ABS_HAT0Y,
        ] {
            assert!(eventos.contains(&eixo(a, 0)), "{a:?}");
        }
        assert_eq!(eventos.len(), 18);
    }

    #[test]
    fn soltar_tudo_esquece_o_direcional_pressionado() {
        let mut t = Tradutor::default();
        t.traduzir(botao(12, true));
        t.traduzir(Evento::SoltarTudo);
        assert_eq!(t.traduzir(botao(13, true)), vec![eixo(Abs::ABS_HAT0X, 1)]);
    }

    #[test]
    fn ids_desconhecidos_sao_ignorados() {
        let mut t = Tradutor::default();
        assert!(t.traduzir(botao(99, true)).is_empty());
        assert!(t.traduzir(Evento::Eixo { id: 99, valor: 1 }).is_empty());
    }
}
