//! Mapa de controle → tecla do modo teclado. Cada controle da página (botões, direcional,
//! gatilhos e as quatro direções de cada analógico) pode apontar para uma tecla ou nenhuma.

use std::collections::{BTreeMap, HashMap};

use evdev::KeyCode;

use crate::teclas::{tecla_por_codigo, tecla_por_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direcao {
    Cima,
    Baixo,
    Esquerda,
    Direita,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Controle {
    /// ids do protocolo: 0..=9 botões, 10..=13 direcional
    Botao(u8),
    /// ids de eixo do protocolo: 4 = LT, 5 = RT
    Gatilho(u8),
    /// 0 = analógico esquerdo, 1 = direito
    Analogico(u8, Direcao),
}

const BOTOES: [&str; 14] = [
    "A", "B", "X", "Y", "LB", "RB", "Select", "Start", "L3", "R3", "Up", "Down", "Left", "Right",
];
const DIRECOES: [Direcao; 4] = [Direcao::Cima, Direcao::Baixo, Direcao::Esquerda, Direcao::Direita];

impl Controle {
    /// Todos os controles, na ordem em que a página os lista.
    pub fn todos() -> Vec<Controle> {
        let botoes = (0..14).map(Controle::Botao);
        let gatilhos = [4, 5].map(Controle::Gatilho);
        let analogicos = [0u8, 1]
            .into_iter()
            .flat_map(|s| DIRECOES.map(|d| Controle::Analogico(s, d)));
        botoes.chain(gatilhos).chain(analogicos).collect()
    }

    pub fn do_botao(id: u8) -> Option<Controle> {
        (id < 14).then_some(Controle::Botao(id))
    }

    pub fn nome(self) -> &'static str {
        use Direcao::*;
        match self {
            Controle::Botao(i) => BOTOES[i as usize],
            Controle::Gatilho(4) => "LT",
            Controle::Gatilho(_) => "RT",
            Controle::Analogico(0, Cima) => "LS_Up",
            Controle::Analogico(0, Baixo) => "LS_Down",
            Controle::Analogico(0, Esquerda) => "LS_Left",
            Controle::Analogico(0, Direita) => "LS_Right",
            Controle::Analogico(_, Cima) => "RS_Up",
            Controle::Analogico(_, Baixo) => "RS_Down",
            Controle::Analogico(_, Esquerda) => "RS_Left",
            Controle::Analogico(_, Direita) => "RS_Right",
        }
    }

    pub fn de_nome(nome: &str) -> Option<Controle> {
        Self::todos().into_iter().find(|c| c.nome() == nome)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Mapa(HashMap<Controle, KeyCode>);

/// Controle → id da tecla. O analógico direito fica livre.
const PADRAO: [(&str, &str); 20] = [
    ("A", "KeyK"),
    ("B", "KeyL"),
    ("X", "KeyJ"),
    ("Y", "KeyI"),
    ("LB", "KeyQ"),
    ("RB", "KeyE"),
    ("Select", "Tab"),
    ("Start", "Enter"),
    ("L3", "KeyC"),
    ("R3", "KeyV"),
    ("Up", "ArrowUp"),
    ("Down", "ArrowDown"),
    ("Left", "ArrowLeft"),
    ("Right", "ArrowRight"),
    ("LT", "ShiftLeft"),
    ("RT", "Space"),
    ("LS_Up", "KeyW"),
    ("LS_Down", "KeyS"),
    ("LS_Left", "KeyA"),
    ("LS_Right", "KeyD"),
];

impl Mapa {
    pub fn padrao() -> Mapa {
        let json = PADRAO.iter().map(|(c, t)| (c.to_string(), t.to_string())).collect();
        Self::de_json(&json).expect("o mapa padrão usa só nomes válidos")
    }

    pub fn tecla(&self, controle: Controle) -> Option<KeyCode> {
        self.0.get(&controle).copied()
    }

    /// Controle → id da tecla; controles sem tecla ficam de fora.
    pub fn para_json(&self) -> BTreeMap<String, String> {
        self.0
            .iter()
            .filter_map(|(c, k)| tecla_por_codigo(*k).map(|t| (c.nome().to_string(), t.id.to_string())))
            .collect()
    }

    pub fn de_json(json: &BTreeMap<String, String>) -> Result<Mapa, String> {
        let mut mapa = HashMap::new();
        for (nome, id) in json {
            let controle = Controle::de_nome(nome).ok_or_else(|| format!("controle desconhecido: {nome}"))?;
            let tecla = tecla_por_id(id).ok_or_else(|| format!("tecla desconhecida: {id}"))?;
            mapa.insert(controle, tecla.codigo);
        }
        Ok(Mapa(mapa))
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use evdev::KeyCode;
    use std::collections::BTreeMap;

    const NOMES: [&str; 24] = [
        "A", "B", "X", "Y", "LB", "RB", "Select", "Start", "L3", "R3", "Up", "Down", "Left",
        "Right", "LT", "RT", "LS_Up", "LS_Down", "LS_Left", "LS_Right", "RS_Up", "RS_Down",
        "RS_Left", "RS_Right",
    ];

    #[test]
    fn controles_na_ordem_combinada_com_a_pagina() {
        // web/src/controls.ts usa estes mesmos ids: mudar um lado exige mudar o outro.
        let nomes: Vec<_> = Controle::todos().iter().map(|c| c.nome()).collect();
        assert_eq!(nomes, NOMES);
    }

    #[test]
    fn nome_e_controle_ida_e_volta() {
        for c in Controle::todos() {
            assert_eq!(Controle::de_nome(c.nome()), Some(c));
        }
        assert_eq!(Controle::de_nome("Z"), None);
    }

    #[test]
    fn controles_do_protocolo() {
        assert_eq!(Controle::do_botao(0), Some(Controle::Botao(0)));
        assert_eq!(Controle::do_botao(13), Some(Controle::Botao(13)));
        assert_eq!(Controle::do_botao(14), None);
    }

    #[test]
    fn padrao_cobre_20_controles_e_deixa_o_analogico_direito_livre() {
        let m = Mapa::padrao();
        assert_eq!(m.tecla(Controle::de_nome("A").unwrap()), Some(KeyCode::KEY_K));
        assert_eq!(m.tecla(Controle::de_nome("Y").unwrap()), Some(KeyCode::KEY_I));
        assert_eq!(m.tecla(Controle::de_nome("LS_Up").unwrap()), Some(KeyCode::KEY_W));
        assert_eq!(m.tecla(Controle::de_nome("Up").unwrap()), Some(KeyCode::KEY_UP));
        assert_eq!(m.tecla(Controle::de_nome("RT").unwrap()), Some(KeyCode::KEY_SPACE));
        assert_eq!(m.tecla(Controle::de_nome("RS_Up").unwrap()), None);
        let mapeados = Controle::todos().into_iter().filter(|c| m.tecla(*c).is_some()).count();
        assert_eq!(mapeados, 20);
    }

    #[test]
    fn json_ida_e_volta() {
        let m = Mapa::padrao();
        let json = m.para_json();
        assert_eq!(json.get("A").map(String::as_str), Some("KeyK"));
        assert!(!json.contains_key("RS_Up"));
        assert_eq!(Mapa::de_json(&json).unwrap(), m);
    }

    #[test]
    fn json_vazio_deixa_tudo_sem_tecla() {
        let m = Mapa::de_json(&BTreeMap::new()).unwrap();
        assert!(Controle::todos().into_iter().all(|c| m.tecla(c).is_none()));
    }

    #[test]
    fn json_com_controle_desconhecido_e_rejeitado() {
        let json = BTreeMap::from([("Turbo".to_string(), "KeyA".to_string())]);
        let erro = Mapa::de_json(&json).unwrap_err();
        assert!(erro.contains("Turbo"), "{erro}");
    }

    #[test]
    fn json_com_tecla_desconhecida_e_rejeitado() {
        let json = BTreeMap::from([("A".to_string(), "Hyper".to_string())]);
        let erro = Mapa::de_json(&json).unwrap_err();
        assert!(erro.contains("Hyper"), "{erro}");
    }

    #[test]
    fn mesma_tecla_em_dois_controles_e_permitida() {
        let json = BTreeMap::from([
            ("A".to_string(), "Space".to_string()),
            ("B".to_string(), "Space".to_string()),
        ]);
        let m = Mapa::de_json(&json).unwrap();
        assert_eq!(m.tecla(Controle::Botao(0)), m.tecla(Controle::Botao(1)));
    }
}
