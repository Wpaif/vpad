//! Guarda o mapa de teclas: em memória (compartilhado com o serviço por um canal `watch`)
//! e, se houver arquivo, em disco, para sobreviver a reinícios.

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::sync::watch;

use crate::mapa::Mapa;

pub struct ArmazemMapa {
    tx: watch::Sender<Arc<Mapa>>,
    arquivo: Option<PathBuf>,
}

impl ArmazemMapa {
    pub fn em_memoria() -> Self {
        Self { tx: watch::channel(Arc::new(Mapa::padrao())).0, arquivo: None }
    }

    /// Carrega o mapa do arquivo; se não existir ou estiver inválido, usa o padrão (sem
    /// apagar o arquivo: ele só é substituído quando o usuário salvar outro mapa).
    pub fn com_arquivo(arquivo: PathBuf) -> Self {
        let mapa = match ler(&arquivo) {
            Ok(Some(m)) => m,
            Ok(None) => Mapa::padrao(),
            Err(e) => {
                eprintln!("ignorando {}: {e}", arquivo.display());
                Mapa::padrao()
            }
        };
        Self { tx: watch::channel(Arc::new(mapa)).0, arquivo: Some(arquivo) }
    }

    pub fn atual(&self) -> Arc<Mapa> {
        self.tx.borrow().clone()
    }

    pub fn assinar(&self) -> watch::Receiver<Arc<Mapa>> {
        self.tx.subscribe()
    }

    /// Grava e só então publica: se o disco falhar, o mapa em uso não muda.
    pub fn definir(&self, mapa: Mapa) -> io::Result<()> {
        if let Some(arquivo) = &self.arquivo {
            gravar(arquivo, &mapa)?;
        }
        self.tx.send_replace(Arc::new(mapa));
        Ok(())
    }
}

fn ler(arquivo: &Path) -> Result<Option<Mapa>, String> {
    let texto = match fs::read_to_string(arquivo) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let json: BTreeMap<String, String> = serde_json::from_str(&texto).map_err(|e| e.to_string())?;
    Mapa::de_json(&json).map(Some)
}

/// Escreve num arquivo temporário e renomeia, para nunca deixar um arquivo pela metade.
fn gravar(arquivo: &Path, mapa: &Mapa) -> io::Result<()> {
    if let Some(pasta) = arquivo.parent() {
        fs::create_dir_all(pasta)?;
    }
    let texto = serde_json::to_string_pretty(&mapa.para_json()).map_err(io::Error::other)?;
    let temporario = arquivo.with_extension("json.tmp");
    fs::write(&temporario, texto)?;
    fs::rename(&temporario, arquivo)
}

/// `$XDG_CONFIG_HOME/vpad/teclado.json`, ou `~/.config/vpad/teclado.json`.
pub fn caminho_da_config(xdg: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    let base = match xdg.filter(|x| !x.is_empty()) {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from(home.filter(|h| !h.is_empty())?).join(".config"),
    };
    Some(base.join("vpad").join("teclado.json"))
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::{collections::BTreeMap, fs};

    /// Diretório temporário só deste teste, removido ao final.
    struct Pasta(PathBuf);
    impl Pasta {
        fn nova(nome: &str) -> Pasta {
            let p = std::env::temp_dir().join(format!("vpad-teste-{}-{nome}", std::process::id()));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            Pasta(p)
        }
        fn arquivo(&self) -> PathBuf {
            self.0.join("vpad").join("teclado.json")
        }
    }
    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn mapa(pares: &[(&str, &str)]) -> Mapa {
        let json: BTreeMap<_, _> = pares.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        Mapa::de_json(&json).unwrap()
    }

    #[test]
    fn em_memoria_comeca_no_padrao_e_notifica_quem_assinou() {
        let a = ArmazemMapa::em_memoria();
        assert_eq!(*a.atual(), Mapa::padrao());

        let mut rx = a.assinar();
        a.definir(mapa(&[("A", "Space")])).unwrap();
        assert!(rx.has_changed().unwrap());
        assert_eq!(**rx.borrow_and_update(), mapa(&[("A", "Space")]));
        assert_eq!(*a.atual(), mapa(&[("A", "Space")]));
    }

    #[test]
    fn grava_no_arquivo_criando_as_pastas_e_recarrega_depois() {
        let pasta = Pasta::nova("grava");
        let a = ArmazemMapa::com_arquivo(pasta.arquivo());
        assert_eq!(*a.atual(), Mapa::padrao(), "sem arquivo, vale o padrão");

        a.definir(mapa(&[("A", "Space"), ("RS_Up", "KeyO")])).unwrap();
        assert!(pasta.arquivo().exists());

        let outra = ArmazemMapa::com_arquivo(pasta.arquivo());
        assert_eq!(*outra.atual(), mapa(&[("A", "Space"), ("RS_Up", "KeyO")]));
    }

    #[test]
    fn arquivo_e_um_json_simples_controle_para_tecla() {
        let pasta = Pasta::nova("formato");
        ArmazemMapa::com_arquivo(pasta.arquivo()).definir(mapa(&[("A", "Space")])).unwrap();
        let texto = fs::read_to_string(pasta.arquivo()).unwrap();
        let json: BTreeMap<String, String> = serde_json::from_str(&texto).unwrap();
        assert_eq!(json, BTreeMap::from([("A".to_string(), "Space".to_string())]));
    }

    #[test]
    fn arquivo_corrompido_ou_invalido_cai_no_padrao_sem_quebrar() {
        for (nome, conteudo) in [("lixo", "isto nao e json"), ("tecla", r#"{"A":"Hyper"}"#)] {
            let pasta = Pasta::nova(nome);
            fs::create_dir_all(pasta.arquivo().parent().unwrap()).unwrap();
            fs::write(pasta.arquivo(), conteudo).unwrap();
            let a = ArmazemMapa::com_arquivo(pasta.arquivo());
            assert_eq!(*a.atual(), Mapa::padrao(), "{nome}");
        }
    }

    #[test]
    fn falha_ao_gravar_devolve_erro_e_nao_muda_o_mapa() {
        let pasta = Pasta::nova("falha");
        // a "pasta" vpad é um arquivo comum: impossível criar o teclado.json dentro dela
        fs::write(pasta.0.join("vpad"), "x").unwrap();
        let a = ArmazemMapa::com_arquivo(pasta.arquivo());
        let rx = a.assinar();

        assert!(a.definir(mapa(&[("A", "Space")])).is_err());
        assert_eq!(*a.atual(), Mapa::padrao());
        assert!(!rx.has_changed().unwrap());
    }

    #[test]
    fn caminho_da_configuracao_segue_o_xdg() {
        assert_eq!(
            caminho_da_config(Some("/x"), Some("/h")),
            Some(PathBuf::from("/x/vpad/teclado.json"))
        );
        assert_eq!(
            caminho_da_config(None, Some("/h")),
            Some(PathBuf::from("/h/.config/vpad/teclado.json"))
        );
        assert_eq!(
            caminho_da_config(Some(""), Some("/h")),
            Some(PathBuf::from("/h/.config/vpad/teclado.json"))
        );
        assert_eq!(caminho_da_config(None, None), None);
    }
}
