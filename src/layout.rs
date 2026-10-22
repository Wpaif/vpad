//! O layout dos botões do controle, guardado no PC: um só, compartilhado pelos aparelhos
//! pareados. Assim ele sobrevive a reinstalar o app da Tela de Início, a um novo pareamento
//! e à diferença entre o Safari e o app (que têm armazenamentos separados no iPhone).
//!
//! O servidor não interpreta o layout, só o valida (formato e limites) antes de guardar:
//! quem pode escrever é um aparelho pareado, e o arquivo nunca deve virar um depósito de
//! dados arbitrários.

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::RwLock,
};

use serde_json::Value;

const CAMPOS: [&str; 4] = ["version", "groups", "hidden", "controls"];
const MAX_GRUPOS: usize = 16;
const MAX_CONTROLES: usize = 32;
const MAX_ID: usize = 24;

fn id_valido(id: &str) -> bool {
    !id.is_empty() && id.len() <= MAX_ID && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Um objeto cujos campos são só os de `campos` (nome, mínimo, máximo), todos números nessa faixa.
fn ajuste_valido(valor: &Value, campos: &[(&str, f64, f64)]) -> Result<(), String> {
    let objeto = valor.as_object().ok_or("o ajuste precisa ser um objeto")?;
    for (nome, v) in objeto {
        let (_, min, max) = campos.iter().find(|(c, _, _)| c == nome).ok_or_else(|| format!("campo desconhecido: {nome}"))?;
        match v.as_f64() {
            Some(n) if n.is_finite() && (*min..=*max).contains(&n) => {}
            _ => return Err(format!("{nome} fora da faixa ou não numérico")),
        }
    }
    Ok(())
}

/// Um objeto `id -> ajuste` com no máximo `max` entradas e ids de formato seguro.
fn mapa_de_ajustes(valor: &Value, max: usize, campos: &[(&str, f64, f64)]) -> Result<(), String> {
    let objeto = valor.as_object().ok_or("esperava um objeto")?;
    if objeto.len() > max {
        return Err("entradas demais".into());
    }
    for (id, ajuste) in objeto {
        if !id_valido(id) {
            return Err(format!("id inválido: {id:?}"));
        }
        ajuste_valido(ajuste, campos)?;
    }
    Ok(())
}

/// Confere o formato do layout e devolve uma cópia pronta para guardar.
pub fn validar(valor: &Value) -> Result<Value, String> {
    let objeto = valor.as_object().ok_or("o layout precisa ser um objeto")?;
    if let Some(extra) = objeto.keys().find(|k| !CAMPOS.contains(&k.as_str())) {
        return Err(format!("campo desconhecido: {extra}"));
    }
    match objeto.get("version").and_then(Value::as_u64) {
        Some(1..=1000) => {}
        _ => return Err("version precisa ser um inteiro de 1 a 1000".into()),
    }
    if let Some(grupos) = objeto.get("groups") {
        mapa_de_ajustes(grupos, MAX_GRUPOS, &[("dx", -1.0, 1.0), ("dy", -1.0, 1.0), ("scale", 0.5, 2.0)])
            .map_err(|e| format!("groups: {e}"))?;
    }
    if let Some(controles) = objeto.get("controls") {
        mapa_de_ajustes(controles, MAX_CONTROLES, &[("dx", -1.0, 1.0), ("dy", -1.0, 1.0)])
            .map_err(|e| format!("controls: {e}"))?;
    }
    if let Some(ocultos) = objeto.get("hidden") {
        let lista = ocultos.as_array().ok_or("hidden precisa ser uma lista")?;
        if lista.len() > MAX_CONTROLES {
            return Err("hidden: itens demais".into());
        }
        if !lista.iter().all(|c| c.as_str().is_some_and(id_valido)) {
            return Err("hidden: só ids de controle".into());
        }
    }
    Ok(valor.clone())
}

pub struct ArmazemLayout {
    atual: RwLock<Option<Value>>,
    arquivo: Option<PathBuf>,
}

impl ArmazemLayout {
    pub fn em_memoria() -> Self {
        Self { atual: RwLock::new(None), arquivo: None }
    }

    /// Carrega o layout do arquivo; se não existir ou estiver inválido, começa sem layout
    /// (sem apagar o arquivo: ele só é substituído quando um aparelho salvar outro).
    pub fn com_arquivo(arquivo: PathBuf) -> Self {
        let atual = match ler(&arquivo) {
            Ok(layout) => layout,
            Err(e) => {
                eprintln!("ignorando {}: {e}", arquivo.display());
                None
            }
        };
        Self { atual: RwLock::new(atual), arquivo: Some(arquivo) }
    }

    pub fn atual(&self) -> Option<Value> {
        self.atual.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Valida, grava e só então publica: se o disco falhar, o layout em uso não muda.
    pub fn definir(&self, valor: &Value) -> Result<Value, ErroLayout> {
        let valido = validar(valor).map_err(ErroLayout::Invalido)?;
        if let Some(arquivo) = &self.arquivo {
            gravar(arquivo, &valido).map_err(ErroLayout::Disco)?;
        }
        *self.atual.write().unwrap_or_else(|e| e.into_inner()) = Some(valido.clone());
        Ok(valido)
    }
}

#[derive(Debug)]
pub enum ErroLayout {
    Invalido(String),
    Disco(io::Error),
}

fn ler(arquivo: &Path) -> Result<Option<Value>, String> {
    let texto = match fs::read_to_string(arquivo) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let json: Value = serde_json::from_str(&texto).map_err(|e| e.to_string())?;
    validar(&json).map(Some)
}

/// Escreve num arquivo temporário e renomeia, para nunca deixar um arquivo pela metade.
fn gravar(arquivo: &Path, valor: &Value) -> io::Result<()> {
    if let Some(pasta) = arquivo.parent() {
        fs::create_dir_all(pasta)?;
    }
    let texto = serde_json::to_string_pretty(valor).map_err(io::Error::other)?;
    let temporario = arquivo.with_extension("json.tmp");
    fs::write(&temporario, texto)?;
    fs::rename(&temporario, arquivo)
}

/// `$XDG_CONFIG_HOME/vpad/layout.json`, ou `~/.config/vpad/layout.json`.
pub fn caminho_do_layout(xdg: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    let base = match xdg.filter(|x| !x.is_empty()) {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from(home.filter(|h| !h.is_empty())?).join(".config"),
    };
    Some(base.join("vpad").join("layout.json"))
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    fn completo() -> Value {
        json!({
            "version": 1,
            "groups": { "face": { "dx": -0.1, "dy": 0.05, "scale": 1.3 }, "menu": { "dx": 0.0, "dy": 0.2, "scale": 0.8 } },
            "hidden": ["A", "StickRight"],
            "controls": { "B": { "dx": 0.04, "dy": -0.1 } }
        })
    }

    fn padrao() -> Value {
        json!({ "version": 1, "groups": {}, "hidden": [] })
    }

    // ---------- validar ----------

    #[test]
    fn aceita_o_layout_padrao_e_um_completo() {
        assert_eq!(validar(&padrao()).unwrap(), padrao());
        assert_eq!(validar(&completo()).unwrap(), completo());
    }

    #[test]
    fn so_aceita_objeto() {
        for v in [json!(null), json!([]), json!("x"), json!(3), json!(true)] {
            assert!(validar(&v).is_err(), "{v}");
        }
    }

    #[test]
    fn a_versao_e_um_inteiro_positivo() {
        for v in [json!(0), json!(-1), json!(1.5), json!("1"), json!(null), json!(100_000)] {
            let mut l = padrao();
            l["version"] = v.clone();
            assert!(validar(&l).is_err(), "{v}");
        }
        let sem = json!({ "groups": {}, "hidden": [] });
        assert!(validar(&sem).is_err());
    }

    #[test]
    fn campo_desconhecido_no_topo_e_recusado() {
        let mut l = padrao();
        l["extra"] = json!(1);
        assert!(validar(&l).is_err());
    }

    #[test]
    fn groups_e_hidden_podem_faltar_mas_nao_ter_tipo_errado() {
        assert!(validar(&json!({ "version": 1 })).is_ok());
        let mut l = padrao();
        l["groups"] = json!([]);
        assert!(validar(&l).is_err());
        let mut l = padrao();
        l["hidden"] = json!({});
        assert!(validar(&l).is_err());
    }

    #[test]
    fn grupo_com_entrada_ruim_e_recusado() {
        for entrada in [
            json!("x"),
            json!([1]),
            json!({ "dx": "1", "dy": 0, "scale": 1 }),
            json!({ "dx": 0, "dy": 0, "scale": 1, "extra": 1 }),
            json!({ "dx": 1.5, "dy": 0, "scale": 1 }),
            json!({ "dx": 0, "dy": -1.5, "scale": 1 }),
            json!({ "dx": 0, "dy": 0, "scale": 0.1 }),
            json!({ "dx": 0, "dy": 0, "scale": 3 }),
            json!({ "dx": null, "dy": 0, "scale": 1 }),
        ] {
            let mut l = padrao();
            l["groups"] = json!({ "face": entrada.clone() });
            assert!(validar(&l).is_err(), "{entrada}");
        }
    }

    #[test]
    fn grupo_pode_ter_so_alguns_campos() {
        let mut l = padrao();
        l["groups"] = json!({ "face": { "dx": 0.1 } });
        assert!(validar(&l).is_ok());
    }

    #[test]
    fn muitos_grupos_ou_ids_estranhos_sao_recusados() {
        let mut l = padrao();
        let muitos: serde_json::Map<String, Value> =
            (0..=MAX_GRUPOS).map(|i| (format!("g{i}"), json!({ "dx": 0 }))).collect();
        l["groups"] = Value::Object(muitos);
        assert!(validar(&l).is_err());

        for id in ["../etc", "a b", "", "é", &"x".repeat(MAX_ID + 1)] {
            let mut l = padrao();
            l["groups"] = json!({ id: { "dx": 0 } });
            assert!(validar(&l).is_err(), "{id:?}");
        }
    }

    #[test]
    fn hidden_so_aceita_poucas_strings_curtas() {
        for h in [json!([1]), json!([null]), json!(["x".repeat(MAX_ID + 1)]), json!(["a b"])] {
            let mut l = padrao();
            l["hidden"] = h.clone();
            assert!(validar(&l).is_err(), "{h}");
        }
        let mut l = padrao();
        l["hidden"] = Value::Array((0..=MAX_CONTROLES).map(|i| json!(format!("C{i}"))).collect());
        assert!(validar(&l).is_err());
    }

    #[test]
    fn controls_segue_as_mesmas_regras_com_dx_e_dy() {
        for entrada in [
            json!({ "dx": 0, "dy": 0, "scale": 1 }),
            json!({ "dx": 2, "dy": 0 }),
            json!({ "dx": "a" }),
            json!(5),
        ] {
            let mut l = padrao();
            l["controls"] = json!({ "A": entrada.clone() });
            assert!(validar(&l).is_err(), "{entrada}");
        }
        let mut l = padrao();
        l["controls"] = json!([]);
        assert!(validar(&l).is_err());
        let mut l = padrao();
        let muitos: serde_json::Map<String, Value> =
            (0..=MAX_CONTROLES).map(|i| (format!("C{i}"), json!({ "dx": 0 }))).collect();
        l["controls"] = Value::Object(muitos);
        assert!(validar(&l).is_err());
    }

    #[test]
    fn os_campos_aceitos_sao_so_os_do_cliente() {
        assert_eq!(CAMPOS, ["version", "groups", "hidden", "controls"]);
    }

    // ---------- armazenamento ----------

    /// Diretório temporário só deste teste, removido ao final.
    struct Pasta(PathBuf);
    impl Pasta {
        fn nova(nome: &str) -> Pasta {
            let p = std::env::temp_dir().join(format!("vpad-layout-{}-{nome}", std::process::id()));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            Pasta(p)
        }
        fn arquivo(&self) -> PathBuf {
            self.0.join("vpad").join("layout.json")
        }
    }
    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn em_memoria_comeca_sem_layout_e_guarda_o_que_receber() {
        let a = ArmazemLayout::em_memoria();
        assert_eq!(a.atual(), None);
        assert_eq!(a.definir(&completo()).unwrap(), completo());
        assert_eq!(a.atual(), Some(completo()));
    }

    #[test]
    fn layout_invalido_nao_muda_nada() {
        let a = ArmazemLayout::em_memoria();
        a.definir(&completo()).unwrap();
        assert!(matches!(a.definir(&json!([1, 2])), Err(ErroLayout::Invalido(_))));
        assert_eq!(a.atual(), Some(completo()));
    }

    #[test]
    fn grava_no_arquivo_criando_as_pastas_e_recarrega_depois() {
        let pasta = Pasta::nova("grava");
        let a = ArmazemLayout::com_arquivo(pasta.arquivo());
        assert_eq!(a.atual(), None, "sem arquivo, sem layout");
        a.definir(&completo()).unwrap();
        assert!(pasta.arquivo().exists());
        assert!(!pasta.arquivo().with_extension("json.tmp").exists(), "o temporário não fica");
        assert_eq!(ArmazemLayout::com_arquivo(pasta.arquivo()).atual(), Some(completo()));
    }

    #[test]
    fn arquivo_corrompido_ou_invalido_vira_sem_layout_e_nao_e_apagado() {
        for (nome, conteudo) in [("lixo", "isto nao e json"), ("formato", r#"{"version":0}"#)] {
            let pasta = Pasta::nova(nome);
            fs::create_dir_all(pasta.arquivo().parent().unwrap()).unwrap();
            fs::write(pasta.arquivo(), conteudo).unwrap();
            assert_eq!(ArmazemLayout::com_arquivo(pasta.arquivo()).atual(), None, "{nome}");
            assert_eq!(fs::read_to_string(pasta.arquivo()).unwrap(), conteudo, "{nome}: o arquivo continua lá");
        }
    }

    #[test]
    fn falha_ao_gravar_devolve_erro_e_nao_muda_o_layout() {
        let pasta = Pasta::nova("falha");
        // a "pasta" vpad é um arquivo comum: impossível criar o layout.json dentro dela
        fs::write(pasta.0.join("vpad"), "x").unwrap();
        let a = ArmazemLayout::com_arquivo(pasta.arquivo());
        assert!(matches!(a.definir(&completo()), Err(ErroLayout::Disco(_))));
        assert_eq!(a.atual(), None);
    }

    #[test]
    fn caminho_segue_o_xdg() {
        assert_eq!(caminho_do_layout(Some("/x"), Some("/h")), Some(PathBuf::from("/x/vpad/layout.json")));
        assert_eq!(caminho_do_layout(None, Some("/h")), Some(PathBuf::from("/h/.config/vpad/layout.json")));
        assert_eq!(caminho_do_layout(Some(""), Some("/h")), Some(PathBuf::from("/h/.config/vpad/layout.json")));
        assert_eq!(caminho_do_layout(None, None), None);
    }
}
