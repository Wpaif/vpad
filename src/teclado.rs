//! Modo teclado: traduz os eventos do celular em teclas, segundo o mapa de controles.

use std::collections::HashSet;

use evdev::KeyCode;

use crate::{
    mapa::{Controle, Direcao, Mapa},
    mapeamento::EventoLinux,
    protocolo::Evento,
};

/// Analógico: aperta a partir de 50% do curso e só solta abaixo de 25%, para não "tremer".
const ANALOGICO_APERTA: i32 = 16384;
const ANALOGICO_SOLTA: i32 = 8192;
/// Gatilho (0..=255): aperta a partir da metade e solta abaixo de um quarto.
const GATILHO_APERTA: i32 = 128;
const GATILHO_SOLTA: i32 = 64;

pub struct TradutorTeclado {
    mapa: Mapa,
    /// controles que o dedo está segurando agora
    seguradas: HashSet<Controle>,
}

impl TradutorTeclado {
    pub fn novo(mapa: Mapa) -> Self {
        Self { mapa, seguradas: HashSet::new() }
    }

    /// Usa um mapa novo, soltando antes as teclas que o mapa antigo mantinha pressionadas.
    pub fn definir_mapa(&mut self, mapa: Mapa) -> Vec<EventoLinux> {
        let soltas = self.soltar_tudo();
        self.mapa = mapa;
        soltas
    }

    pub fn traduzir(&mut self, evento: Evento) -> Vec<EventoLinux> {
        match evento {
            Evento::Botao { id, pressionado } => Controle::do_botao(id)
                .map(|c| self.mudar(c, pressionado))
                .unwrap_or_default(),
            Evento::Eixo { id, valor } => self.eixo(id, valor as i32),
            Evento::SoltarTudo => self.soltar_tudo(),
            Evento::Ping | Evento::TrocarPerfil(_) => vec![],
        }
    }

    fn tecla_ativa(&self, tecla: KeyCode) -> bool {
        self.seguradas.iter().any(|c| self.mapa.tecla(*c) == Some(tecla))
    }

    /// Teclas hoje pressionadas, em ordem estável.
    fn teclas_ativas(&self) -> Vec<KeyCode> {
        let mut teclas: Vec<_> = self.seguradas.iter().filter_map(|c| self.mapa.tecla(*c)).collect();
        teclas.sort_by_key(|k| k.0);
        teclas.dedup();
        teclas
    }

    fn soltar_tudo(&mut self) -> Vec<EventoLinux> {
        let soltas = self.teclas_ativas().into_iter().map(|k| EventoLinux::tecla(k, 0)).collect();
        self.seguradas.clear();
        soltas
    }

    /// Registra o controle como segurado ou solto e emite o evento só se a tecla mudou de
    /// estado (duas entradas na mesma tecla só a soltam quando ambas soltam).
    fn mudar(&mut self, controle: Controle, ativo: bool) -> Vec<EventoLinux> {
        let Some(tecla) = self.mapa.tecla(controle) else {
            if ativo { self.seguradas.insert(controle) } else { self.seguradas.remove(&controle) };
            return vec![];
        };
        let antes = self.tecla_ativa(tecla);
        if ativo { self.seguradas.insert(controle) } else { self.seguradas.remove(&controle) };
        let depois = self.tecla_ativa(tecla);
        if antes == depois {
            vec![]
        } else {
            vec![EventoLinux::tecla(tecla, depois as i32)]
        }
    }

    fn eixo(&mut self, id: u8, valor: i32) -> Vec<EventoLinux> {
        match id {
            0..=3 => {
                let lado = id / 2;
                let (negativo, positivo) = if id % 2 == 1 {
                    (Direcao::Cima, Direcao::Baixo)
                } else {
                    (Direcao::Esquerda, Direcao::Direita)
                };
                let neg = Controle::Analogico(lado, negativo);
                let pos = Controle::Analogico(lado, positivo);
                let mut eventos = self.mudar(neg, self.passou(neg, -valor));
                eventos.extend(self.mudar(pos, self.passou(pos, valor)));
                // ao atravessar o centro, solta a direção antiga antes de apertar a nova
                eventos.sort_by_key(|e| e.valor);
                eventos
            }
            4 | 5 => {
                let gatilho = Controle::Gatilho(id);
                let ativo = if self.seguradas.contains(&gatilho) {
                    valor >= GATILHO_SOLTA
                } else {
                    valor >= GATILHO_APERTA
                };
                self.mudar(gatilho, ativo)
            }
            _ => vec![],
        }
    }

    /// O analógico está além do limiar nesta direção? (com histerese)
    fn passou(&self, controle: Controle, valor: i32) -> bool {
        if self.seguradas.contains(&controle) {
            valor > ANALOGICO_SOLTA
        } else {
            valor >= ANALOGICO_APERTA
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::mapa::Mapa;
    use evdev::KeyCode;
    use std::collections::BTreeMap;

    fn t() -> TradutorTeclado {
        TradutorTeclado::novo(Mapa::padrao())
    }
    fn tecla(k: KeyCode, v: i32) -> EventoLinux {
        EventoLinux { tipo: evdev::EventType::KEY.0, codigo: k.0, valor: v }
    }
    fn botao(id: u8, pressionado: bool) -> Evento {
        Evento::Botao { id, pressionado }
    }
    fn eixo(id: u8, valor: i16) -> Evento {
        Evento::Eixo { id, valor }
    }
    fn mapa(pares: &[(&str, &str)]) -> Mapa {
        let json: BTreeMap<_, _> = pares.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        Mapa::de_json(&json).unwrap()
    }

    #[test]
    fn botao_vira_tecla_do_mapa() {
        let mut t = t();
        assert_eq!(t.traduzir(botao(0, true)), vec![tecla(KeyCode::KEY_K, 1)]);
        assert_eq!(t.traduzir(botao(0, false)), vec![tecla(KeyCode::KEY_K, 0)]);
    }

    #[test]
    fn direcional_vira_setas() {
        let mut t = t();
        assert_eq!(t.traduzir(botao(10, true)), vec![tecla(KeyCode::KEY_UP, 1)]);
        assert_eq!(t.traduzir(botao(13, true)), vec![tecla(KeyCode::KEY_RIGHT, 1)]);
    }

    #[test]
    fn repetir_o_mesmo_estado_nao_gera_evento() {
        let mut t = t();
        t.traduzir(botao(0, true));
        assert!(t.traduzir(botao(0, true)).is_empty());
        t.traduzir(botao(0, false));
        assert!(t.traduzir(botao(0, false)).is_empty());
    }

    #[test]
    fn controle_sem_tecla_nao_gera_evento() {
        let mut t = TradutorTeclado::novo(mapa(&[]));
        assert!(t.traduzir(botao(0, true)).is_empty());
        assert!(t.traduzir(eixo(0, 30000)).is_empty());
    }

    #[test]
    fn gatilho_aperta_a_partir_da_metade_e_solta_com_histerese() {
        let mut t = t();
        assert!(t.traduzir(eixo(5, 100)).is_empty()); // abaixo do limiar
        assert_eq!(t.traduzir(eixo(5, 255)), vec![tecla(KeyCode::KEY_SPACE, 1)]);
        assert!(t.traduzir(eixo(5, 100)).is_empty()); // ainda segurando
        assert_eq!(t.traduzir(eixo(5, 30)), vec![tecla(KeyCode::KEY_SPACE, 0)]);
        assert_eq!(t.traduzir(eixo(4, 255)), vec![tecla(KeyCode::KEY_LEFTSHIFT, 1)]);
    }

    #[test]
    fn analogico_esquerdo_vira_wasd() {
        let mut t = t();
        assert_eq!(t.traduzir(eixo(0, 20000)), vec![tecla(KeyCode::KEY_D, 1)]);
        assert_eq!(t.traduzir(eixo(0, 0)), vec![tecla(KeyCode::KEY_D, 0)]);
        assert_eq!(t.traduzir(eixo(0, -20000)), vec![tecla(KeyCode::KEY_A, 1)]);
        t.traduzir(eixo(0, 0));
        // y para cima é negativo na página
        assert_eq!(t.traduzir(eixo(1, -20000)), vec![tecla(KeyCode::KEY_W, 1)]);
        t.traduzir(eixo(1, 0));
        assert_eq!(t.traduzir(eixo(1, 20000)), vec![tecla(KeyCode::KEY_S, 1)]);
    }

    #[test]
    fn analogico_tem_zona_morta_e_histerese() {
        let mut t = t();
        assert!(t.traduzir(eixo(0, 10000)).is_empty()); // entre 25% e 50%: não aperta
        t.traduzir(eixo(0, 20000));
        assert!(t.traduzir(eixo(0, 10000)).is_empty()); // já apertado: continua
        assert_eq!(t.traduzir(eixo(0, 5000)), vec![tecla(KeyCode::KEY_D, 0)]);
    }

    #[test]
    fn atravessar_o_centro_solta_uma_direcao_e_aperta_a_outra() {
        let mut t = t();
        t.traduzir(eixo(0, 20000));
        assert_eq!(
            t.traduzir(eixo(0, -20000)),
            vec![tecla(KeyCode::KEY_D, 0), tecla(KeyCode::KEY_A, 1)]
        );
    }

    #[test]
    fn analogico_direito_usa_o_proprio_mapa() {
        let mut t = TradutorTeclado::novo(mapa(&[("RS_Right", "KeyP"), ("RS_Up", "KeyO")]));
        assert_eq!(t.traduzir(eixo(2, 30000)), vec![tecla(KeyCode::KEY_P, 1)]);
        assert_eq!(t.traduzir(eixo(3, -30000)), vec![tecla(KeyCode::KEY_O, 1)]);
    }

    #[test]
    fn duas_entradas_na_mesma_tecla_so_soltam_quando_ambas_soltam() {
        let mut t = TradutorTeclado::novo(mapa(&[("A", "Space"), ("B", "Space")]));
        assert_eq!(t.traduzir(botao(0, true)), vec![tecla(KeyCode::KEY_SPACE, 1)]);
        assert!(t.traduzir(botao(1, true)).is_empty());
        assert!(t.traduzir(botao(0, false)).is_empty());
        assert_eq!(t.traduzir(botao(1, false)), vec![tecla(KeyCode::KEY_SPACE, 0)]);
    }

    #[test]
    fn soltar_tudo_solta_so_o_que_esta_pressionado() {
        let mut t = t();
        t.traduzir(botao(0, true));
        t.traduzir(eixo(0, 20000));
        let mut eventos = t.traduzir(Evento::SoltarTudo);
        eventos.sort_by_key(|e| e.codigo);
        assert_eq!(eventos, vec![tecla(KeyCode::KEY_D, 0), tecla(KeyCode::KEY_K, 0)]);
        assert!(t.traduzir(Evento::SoltarTudo).is_empty());
    }

    #[test]
    fn trocar_o_mapa_solta_as_teclas_seguradas_no_mapa_antigo() {
        let mut t = t();
        t.traduzir(botao(0, true));
        let soltas = t.definir_mapa(mapa(&[("A", "Space")]));
        assert_eq!(soltas, vec![tecla(KeyCode::KEY_K, 0)]);
        // o dedo ainda estava no botão: soltar agora não emite nada
        assert!(t.traduzir(botao(0, false)).is_empty());
        // e o próximo toque já usa o mapa novo
        assert_eq!(t.traduzir(botao(0, true)), vec![tecla(KeyCode::KEY_SPACE, 1)]);
    }

    #[test]
    fn ping_e_troca_de_perfil_nao_geram_eventos() {
        let mut t = t();
        assert!(t.traduzir(Evento::Ping).is_empty());
        assert!(t.traduzir(Evento::TrocarPerfil(1)).is_empty());
    }

    #[test]
    fn ids_desconhecidos_sao_ignorados() {
        let mut t = t();
        assert!(t.traduzir(botao(99, true)).is_empty());
        assert!(t.traduzir(eixo(99, 30000)).is_empty());
    }
}
