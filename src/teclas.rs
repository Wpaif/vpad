//! Teclas que o controle virtual pode emitir. O `id` é o mesmo nome que o navegador dá em
//! `KeyboardEvent.code`, e é o que a página e o arquivo de configuração usam.

use evdev::KeyCode;

#[derive(Debug, PartialEq, Eq)]
pub struct Tecla {
    pub id: &'static str,
    pub codigo: KeyCode,
    /// texto curto para mostrar no botão
    pub rotulo: &'static str,
    pub grupo: &'static str,
}

const fn t(id: &'static str, codigo: KeyCode, rotulo: &'static str, grupo: &'static str) -> Tecla {
    Tecla { id, codigo, rotulo, grupo }
}

pub const TECLAS: &[Tecla] = &[
    t("KeyA", KeyCode::KEY_A, "A", "Letras"),
    t("KeyB", KeyCode::KEY_B, "B", "Letras"),
    t("KeyC", KeyCode::KEY_C, "C", "Letras"),
    t("KeyD", KeyCode::KEY_D, "D", "Letras"),
    t("KeyE", KeyCode::KEY_E, "E", "Letras"),
    t("KeyF", KeyCode::KEY_F, "F", "Letras"),
    t("KeyG", KeyCode::KEY_G, "G", "Letras"),
    t("KeyH", KeyCode::KEY_H, "H", "Letras"),
    t("KeyI", KeyCode::KEY_I, "I", "Letras"),
    t("KeyJ", KeyCode::KEY_J, "J", "Letras"),
    t("KeyK", KeyCode::KEY_K, "K", "Letras"),
    t("KeyL", KeyCode::KEY_L, "L", "Letras"),
    t("KeyM", KeyCode::KEY_M, "M", "Letras"),
    t("KeyN", KeyCode::KEY_N, "N", "Letras"),
    t("KeyO", KeyCode::KEY_O, "O", "Letras"),
    t("KeyP", KeyCode::KEY_P, "P", "Letras"),
    t("KeyQ", KeyCode::KEY_Q, "Q", "Letras"),
    t("KeyR", KeyCode::KEY_R, "R", "Letras"),
    t("KeyS", KeyCode::KEY_S, "S", "Letras"),
    t("KeyT", KeyCode::KEY_T, "T", "Letras"),
    t("KeyU", KeyCode::KEY_U, "U", "Letras"),
    t("KeyV", KeyCode::KEY_V, "V", "Letras"),
    t("KeyW", KeyCode::KEY_W, "W", "Letras"),
    t("KeyX", KeyCode::KEY_X, "X", "Letras"),
    t("KeyY", KeyCode::KEY_Y, "Y", "Letras"),
    t("KeyZ", KeyCode::KEY_Z, "Z", "Letras"),
    t("Digit0", KeyCode::KEY_0, "0", "Números"),
    t("Digit1", KeyCode::KEY_1, "1", "Números"),
    t("Digit2", KeyCode::KEY_2, "2", "Números"),
    t("Digit3", KeyCode::KEY_3, "3", "Números"),
    t("Digit4", KeyCode::KEY_4, "4", "Números"),
    t("Digit5", KeyCode::KEY_5, "5", "Números"),
    t("Digit6", KeyCode::KEY_6, "6", "Números"),
    t("Digit7", KeyCode::KEY_7, "7", "Números"),
    t("Digit8", KeyCode::KEY_8, "8", "Números"),
    t("Digit9", KeyCode::KEY_9, "9", "Números"),
    t("ArrowUp", KeyCode::KEY_UP, "↑", "Setas"),
    t("ArrowDown", KeyCode::KEY_DOWN, "↓", "Setas"),
    t("ArrowLeft", KeyCode::KEY_LEFT, "←", "Setas"),
    t("ArrowRight", KeyCode::KEY_RIGHT, "→", "Setas"),
    t("F1", KeyCode::KEY_F1, "F1", "Funções"),
    t("F2", KeyCode::KEY_F2, "F2", "Funções"),
    t("F3", KeyCode::KEY_F3, "F3", "Funções"),
    t("F4", KeyCode::KEY_F4, "F4", "Funções"),
    t("F5", KeyCode::KEY_F5, "F5", "Funções"),
    t("F6", KeyCode::KEY_F6, "F6", "Funções"),
    t("F7", KeyCode::KEY_F7, "F7", "Funções"),
    t("F8", KeyCode::KEY_F8, "F8", "Funções"),
    t("F9", KeyCode::KEY_F9, "F9", "Funções"),
    t("F10", KeyCode::KEY_F10, "F10", "Funções"),
    t("F11", KeyCode::KEY_F11, "F11", "Funções"),
    t("F12", KeyCode::KEY_F12, "F12", "Funções"),
    t("Space", KeyCode::KEY_SPACE, "Espaço", "Especiais"),
    t("Enter", KeyCode::KEY_ENTER, "Enter", "Especiais"),
    t("Escape", KeyCode::KEY_ESC, "Esc", "Especiais"),
    t("Tab", KeyCode::KEY_TAB, "Tab", "Especiais"),
    t("Backspace", KeyCode::KEY_BACKSPACE, "Apagar", "Especiais"),
    t("ShiftLeft", KeyCode::KEY_LEFTSHIFT, "Shift", "Especiais"),
    t("ShiftRight", KeyCode::KEY_RIGHTSHIFT, "R Shift", "Especiais"),
    t("ControlLeft", KeyCode::KEY_LEFTCTRL, "Ctrl", "Especiais"),
    t("ControlRight", KeyCode::KEY_RIGHTCTRL, "R Ctrl", "Especiais"),
    t("AltLeft", KeyCode::KEY_LEFTALT, "Alt", "Especiais"),
    t("AltRight", KeyCode::KEY_RIGHTALT, "R Alt", "Especiais"),
    t("Delete", KeyCode::KEY_DELETE, "Del", "Especiais"),
    t("Insert", KeyCode::KEY_INSERT, "Ins", "Especiais"),
    t("Home", KeyCode::KEY_HOME, "Home", "Especiais"),
    t("End", KeyCode::KEY_END, "End", "Especiais"),
    t("PageUp", KeyCode::KEY_PAGEUP, "PgUp", "Especiais"),
    t("PageDown", KeyCode::KEY_PAGEDOWN, "PgDn", "Especiais"),
    t("Comma", KeyCode::KEY_COMMA, ",", "Símbolos"),
    t("Period", KeyCode::KEY_DOT, ".", "Símbolos"),
    t("Slash", KeyCode::KEY_SLASH, "/", "Símbolos"),
    t("Semicolon", KeyCode::KEY_SEMICOLON, ";", "Símbolos"),
    t("Quote", KeyCode::KEY_APOSTROPHE, "'", "Símbolos"),
    t("BracketLeft", KeyCode::KEY_LEFTBRACE, "[", "Símbolos"),
    t("BracketRight", KeyCode::KEY_RIGHTBRACE, "]", "Símbolos"),
    t("Backslash", KeyCode::KEY_BACKSLASH, "\\", "Símbolos"),
    t("Minus", KeyCode::KEY_MINUS, "-", "Símbolos"),
    t("Equal", KeyCode::KEY_EQUAL, "=", "Símbolos"),
    t("Backquote", KeyCode::KEY_GRAVE, "`", "Símbolos"),
];

pub fn tecla_por_id(id: &str) -> Option<&'static Tecla> {
    TECLAS.iter().find(|t| t.id == id)
}

pub fn tecla_por_codigo(codigo: KeyCode) -> Option<&'static Tecla> {
    TECLAS.iter().find(|t| t.codigo == codigo)
}

#[cfg(test)]
mod testes {
    use super::*;
    use evdev::KeyCode;
    use std::collections::HashSet;

    #[test]
    fn ids_e_codigos_sao_unicos() {
        let ids: HashSet<_> = TECLAS.iter().map(|t| t.id).collect();
        let codigos: HashSet<_> = TECLAS.iter().map(|t| t.codigo.0).collect();
        assert_eq!(ids.len(), TECLAS.len());
        assert_eq!(codigos.len(), TECLAS.len());
    }

    #[test]
    fn encontra_pelo_id_e_pelo_codigo() {
        assert_eq!(tecla_por_id("KeyA").map(|t| t.codigo), Some(KeyCode::KEY_A));
        assert!(tecla_por_id("KeyÇ").is_none());
        assert_eq!(tecla_por_codigo(KeyCode::KEY_A).map(|t| t.id), Some("KeyA"));
        assert!(tecla_por_codigo(KeyCode::BTN_SOUTH).is_none());
    }

    #[test]
    fn teclas_comuns_apontam_para_o_codigo_certo() {
        let casos = [
            ("Space", KeyCode::KEY_SPACE),
            ("Enter", KeyCode::KEY_ENTER),
            ("Escape", KeyCode::KEY_ESC),
            ("Tab", KeyCode::KEY_TAB),
            ("Backspace", KeyCode::KEY_BACKSPACE),
            ("ShiftLeft", KeyCode::KEY_LEFTSHIFT),
            ("ShiftRight", KeyCode::KEY_RIGHTSHIFT),
            ("ControlLeft", KeyCode::KEY_LEFTCTRL),
            ("AltLeft", KeyCode::KEY_LEFTALT),
            ("Digit0", KeyCode::KEY_0),
            ("Digit1", KeyCode::KEY_1),
            ("ArrowUp", KeyCode::KEY_UP),
            ("ArrowDown", KeyCode::KEY_DOWN),
            ("ArrowLeft", KeyCode::KEY_LEFT),
            ("ArrowRight", KeyCode::KEY_RIGHT),
            ("F1", KeyCode::KEY_F1),
            ("F12", KeyCode::KEY_F12),
            ("Comma", KeyCode::KEY_COMMA),
            ("Period", KeyCode::KEY_DOT),
            ("Slash", KeyCode::KEY_SLASH),
        ];
        for (id, codigo) in casos {
            assert_eq!(tecla_por_id(id).map(|t| t.codigo), Some(codigo), "{id}");
        }
    }

    #[test]
    fn grupos_tem_o_tamanho_esperado() {
        let n = |g: &str| TECLAS.iter().filter(|t| t.grupo == g).count();
        assert_eq!(n("Letras"), 26);
        assert_eq!(n("Números"), 10);
        assert_eq!(n("Setas"), 4);
        assert_eq!(n("Funções"), 12);
        assert!(n("Especiais") >= 10);
    }

    #[test]
    fn rotulos_curtos_para_mostrar_nos_botoes() {
        let r = |id| tecla_por_id(id).unwrap().rotulo;
        assert_eq!(r("KeyA"), "A");
        assert_eq!(r("Digit1"), "1");
        assert_eq!(r("ArrowUp"), "↑");
        assert_eq!(r("Space"), "Espaço");
        assert_eq!(r("ShiftLeft"), "Shift");
        assert_eq!(r("ControlLeft"), "Ctrl");
        assert_eq!(r("Enter"), "Enter");
    }

    #[test]
    fn todo_id_segue_o_padrao_de_keyboardevent_code() {
        // a página usa o mesmo nome que o navegador dá em KeyboardEvent.code
        for t in TECLAS {
            assert!(t.id.chars().all(|c| c.is_ascii_alphanumeric()), "{}", t.id);
        }
    }
}
