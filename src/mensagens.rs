//! O idioma do terminal do dono do PC (a página e o celular têm o próprio mecanismo).
//! Por ora são dois: português quando o sistema está em português, inglês nos demais casos.
//! `VPAD_LANG` (por exemplo `VPAD_LANG=pt`) força um deles.
//!
//! Cada texto fica ao lado da sua tradução, no ponto em que é usado:
//! `tr!("pareamento fechado", "pairing closed")` (com argumentos, como `format!`) ou
//! `txt("pareamento fechado", "pairing closed")` para um texto fixo. Os dois textos são
//! conferidos pelo compilador (marcadores e argumentos).

use std::{
    cell::Cell,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::idiomas::Idioma;

/// 0 = português; 1 = inglês. Escolhido uma vez no `main`; sem isso vale o português, que é o que
/// os testes esperam.
static ATUAL: AtomicU8 = AtomicU8::new(0);

thread_local! {
    /// Só para os testes: troca o idioma na thread atual, sem afetar os outros testes em paralelo.
    static SOBREPOSTO: Cell<Option<Idioma>> = const { Cell::new(None) };
}

/// O idioma do terminal: só `PtBr` ou `En`.
pub fn idioma_do_terminal() -> Idioma {
    if let Some(i) = SOBREPOSTO.with(Cell::get) {
        return i;
    }
    de_u8(ATUAL.load(Ordering::Relaxed))
}

/// O que se guarda no estado global: só o português é 0; qualquer outro idioma vira inglês (1).
fn para_u8(idioma: Idioma) -> u8 {
    u8::from(idioma != Idioma::PtBr)
}

fn de_u8(valor: u8) -> Idioma {
    if valor == 0 { Idioma::PtBr } else { Idioma::En }
}

/// Fixa o idioma do terminal. Qualquer idioma que não seja o português vira inglês.
pub fn definir_idioma_do_terminal(idioma: Idioma) {
    ATUAL.store(para_u8(idioma), Ordering::Relaxed);
}

pub fn em_portugues() -> bool {
    idioma_do_terminal() == Idioma::PtBr
}

/// Roda `f` com o terminal no idioma pedido (só nesta thread). Para os testes.
pub fn com_idioma<R>(idioma: Idioma, f: impl FnOnce() -> R) -> R {
    let anterior = SOBREPOSTO.with(|s| s.replace(Some(idioma)));
    let resultado = f();
    SOBREPOSTO.with(|s| s.set(anterior));
    resultado
}

/// Texto fixo: o em português ou o em inglês, conforme o idioma do terminal.
pub fn txt(pt: &'static str, en: &'static str) -> &'static str {
    if em_portugues() { pt } else { en }
}

/// Como `format!`, com o texto nos dois idiomas: `tr!("código {x}", "code {x}")`.
#[macro_export]
macro_rules! tr {
    ($pt:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        if $crate::mensagens::em_portugues() {
            format!($pt $(, $arg)*)
        } else {
            format!($en $(, $arg)*)
        }
    };
}

/// O idioma do terminal segundo as variáveis de ambiente: `VPAD_LANG`; depois `LC_ALL`,
/// `LC_MESSAGES` e `LANG` (a primeira que estiver preenchida); com o locale em outra língua que
/// não `C`/`POSIX`, o primeiro item de `LANGUAGE` tem a palavra final. Português só se o escolhido
/// começar com `pt`; qualquer outra coisa, inclusive nada, é inglês.
pub fn detectar(ambiente: impl Fn(&str) -> Option<String>) -> Idioma {
    let preenchida = |nome: &str| ambiente(nome).filter(|v| !v.trim().is_empty());
    // `pt_BR.UTF-8` / `pt_BR@euro` → `pt-br`: só o idioma e o país importam
    let normalizar = |v: &str| {
        let base = v.trim().split(['.', '@']).next().unwrap_or("");
        base.replace('_', "-").to_ascii_lowercase()
    };
    let decide = |valor: &str| if valor.starts_with("pt") { Idioma::PtBr } else { Idioma::En };

    if let Some(forcado) = preenchida("VPAD_LANG").map(|v| normalizar(&v)) {
        if let Some(idioma) = Idioma::por_etiqueta(&forcado) {
            return decide_idioma(idioma);
        }
    }
    let Some(locale) = ["LC_ALL", "LC_MESSAGES", "LANG"].into_iter().find_map(preenchida).map(|v| normalizar(&v)) else {
        return Idioma::En;
    };
    if locale == "c" || locale == "posix" {
        return Idioma::En; // o locale "neutro" ignora até o LANGUAGE
    }
    match preenchida("LANGUAGE").and_then(|v| v.split(':').next().map(normalizar)) {
        Some(primeiro) if !primeiro.is_empty() => decide(&primeiro),
        _ => decide(&locale),
    }
}

fn decide_idioma(idioma: Idioma) -> Idioma {
    if idioma == Idioma::PtBr { Idioma::PtBr } else { Idioma::En }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashMap;

    fn amb<'a>(pares: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        let m: HashMap<&str, &str> = pares.iter().copied().collect();
        move |k| m.get(k).map(|v| v.to_string())
    }

    #[test]
    fn sem_nenhuma_variavel_vale_o_ingles() {
        assert_eq!(detectar(amb(&[])), Idioma::En);
    }

    #[test]
    fn sistema_em_portugues_fica_em_portugues() {
        for v in ["pt_BR.UTF-8", "pt_BR", "pt_PT.UTF-8", "pt", "PT_br.utf8"] {
            assert_eq!(detectar(amb(&[("LANG", v)])), Idioma::PtBr, "{v}");
        }
    }

    #[test]
    fn qualquer_outro_idioma_e_c_e_posix_viram_ingles() {
        for v in ["en_US.UTF-8", "es_ES.UTF-8", "ja_JP.UTF-8", "C", "POSIX", "C.UTF-8", "xx"] {
            assert_eq!(detectar(amb(&[("LANG", v)])), Idioma::En, "{v}");
        }
    }

    #[test]
    fn a_primeira_variavel_preenchida_vence_na_ordem_lc_all_lc_messages_lang() {
        assert_eq!(detectar(amb(&[("LC_ALL", "en_US.UTF-8"), ("LANG", "pt_BR.UTF-8")])), Idioma::En);
        assert_eq!(detectar(amb(&[("LC_ALL", "pt_BR.UTF-8"), ("LANG", "en_US.UTF-8")])), Idioma::PtBr);
        assert_eq!(detectar(amb(&[("LC_MESSAGES", "pt_BR.UTF-8"), ("LANG", "en_US.UTF-8")])), Idioma::PtBr);
        assert_eq!(detectar(amb(&[("LC_ALL", "pt_BR.UTF-8"), ("LC_MESSAGES", "en_US.UTF-8")])), Idioma::PtBr);
    }

    #[test]
    fn variavel_vazia_e_ignorada() {
        assert_eq!(detectar(amb(&[("LC_ALL", ""), ("LANG", "pt_BR.UTF-8")])), Idioma::PtBr);
        assert_eq!(detectar(amb(&[("LC_ALL", ""), ("LC_MESSAGES", ""), ("LANG", "")])), Idioma::En);
    }

    #[test]
    fn language_decide_quando_o_locale_nao_e_c() {
        assert_eq!(detectar(amb(&[("LANG", "en_US.UTF-8"), ("LANGUAGE", "pt_BR:en")])), Idioma::PtBr);
        assert_eq!(detectar(amb(&[("LANG", "pt_BR.UTF-8"), ("LANGUAGE", "en:pt_BR")])), Idioma::En);
        assert_eq!(detectar(amb(&[("LANG", "pt_BR.UTF-8"), ("LANGUAGE", "")])), Idioma::PtBr);
    }

    #[test]
    fn language_e_ignorada_com_locale_c() {
        assert_eq!(detectar(amb(&[("LANG", "C"), ("LANGUAGE", "pt_BR")])), Idioma::En);
        assert_eq!(detectar(amb(&[("LANGUAGE", "pt_BR")])), Idioma::En);
    }

    #[test]
    fn vpad_lang_vence_tudo() {
        assert_eq!(detectar(amb(&[("VPAD_LANG", "pt"), ("LANG", "en_US.UTF-8")])), Idioma::PtBr);
        assert_eq!(detectar(amb(&[("VPAD_LANG", "en"), ("LANG", "pt_BR.UTF-8")])), Idioma::En);
        assert_eq!(detectar(amb(&[("VPAD_LANG", "PT-br"), ("LC_ALL", "en_US")])), Idioma::PtBr);
    }

    #[test]
    fn vpad_lang_invalido_ou_vazio_e_ignorado() {
        assert_eq!(detectar(amb(&[("VPAD_LANG", "klingon"), ("LANG", "pt_BR.UTF-8")])), Idioma::PtBr);
        assert_eq!(detectar(amb(&[("VPAD_LANG", ""), ("LANG", "en_US")])), Idioma::En);
    }

    #[test]
    fn o_padrao_do_programa_e_portugues_para_os_testes() {
        assert_eq!(idioma_do_terminal(), Idioma::PtBr);
        assert!(em_portugues());
    }

    #[test]
    fn com_idioma_troca_so_dentro_do_bloco_e_so_nesta_thread() {
        com_idioma(Idioma::En, || {
            assert_eq!(idioma_do_terminal(), Idioma::En);
            assert!(!em_portugues());
            // outra thread não enxerga a troca
            std::thread::spawn(|| assert_eq!(idioma_do_terminal(), Idioma::PtBr)).join().unwrap();
        });
        assert_eq!(idioma_do_terminal(), Idioma::PtBr);
    }

    #[test]
    fn com_idioma_aninhado_restaura_o_anterior() {
        com_idioma(Idioma::En, || {
            com_idioma(Idioma::PtBr, || assert_eq!(idioma_do_terminal(), Idioma::PtBr));
            assert_eq!(idioma_do_terminal(), Idioma::En);
        });
    }

    #[test]
    fn txt_escolhe_o_texto_do_idioma() {
        assert_eq!(txt("fechado", "closed"), "fechado");
        com_idioma(Idioma::En, || assert_eq!(txt("fechado", "closed"), "closed"));
    }

    #[test]
    fn tr_formata_o_texto_do_idioma_com_os_mesmos_argumentos() {
        let (n, nome) = (7, "pc");
        assert_eq!(tr!("{n} aparelhos em {nome}", "{n} devices on {nome}"), "7 aparelhos em pc");
        com_idioma(Idioma::En, || {
            assert_eq!(tr!("{n} aparelhos em {nome}", "{n} devices on {nome}"), "7 devices on pc");
            assert_eq!(tr!("{} de {}", "{} of {}", 1, 2), "1 of 2");
            assert_eq!(tr!("só texto", "plain text"), "plain text");
        });
    }

    #[test]
    fn tr_aceita_ordem_diferente_dos_argumentos_nos_dois_idiomas() {
        com_idioma(Idioma::En, || assert_eq!(tr!("{0} e {1}", "{1} and {0}", "a", "b"), "b and a"));
        assert_eq!(tr!("{0} e {1}", "{1} and {0}", "a", "b"), "a e b");
    }

    #[test]
    fn o_estado_global_guarda_o_portugues_e_trata_todo_o_resto_como_ingles() {
        // (a conversão é pura: não se mexe no estado global, que os outros testes em paralelo leem)
        assert_eq!(de_u8(para_u8(Idioma::PtBr)), Idioma::PtBr);
        for outro in [Idioma::En, Idioma::Es, Idioma::ZhCn, Idioma::Ja, Idioma::Ko] {
            assert_eq!(de_u8(para_u8(outro)), Idioma::En, "{outro:?}");
        }
    }
}
