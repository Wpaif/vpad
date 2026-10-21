//! Controle de acesso: só quem tem o token conecta, e só um aparelho por vez controla.
//! O primeiro a entrar vira o dono; os demais ficam de fora até ele sair. O mesmo aparelho
//! pode reassumir na hora (por exemplo, depois de uma queda do Wi-Fi), derrubando a conexão
//! antiga que o servidor ainda não percebeu que morreu.

use std::{
    fs,
    io::{self, Read, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::sync::oneshot;

pub struct Acesso {
    token: String,
    inatividade: Duration,
    estado: Mutex<Estado>,
}

#[derive(Default)]
struct Estado {
    dono: Option<Dono>,
    proxima_geracao: u64,
}

struct Dono {
    cliente: String,
    /// distingue conexões do mesmo aparelho: só a geração atual pode soltar a vaga
    geracao: u64,
    derrubar: Option<oneshot::Sender<()>>,
}

pub enum Entrada {
    Dono(Vaga),
    Ocupado,
}

/// A vaga de dono do controle. Ao ser largada, libera o controle (se ainda for dela).
pub struct Vaga {
    acesso: Arc<Acesso>,
    geracao: u64,
    liberada: bool,
    /// true quando esta conexão tomou o lugar de outra do mesmo aparelho
    pub substituiu: bool,
    /// resolve quando outra conexão do mesmo aparelho toma o lugar desta
    pub derrubada: oneshot::Receiver<()>,
}

impl Acesso {
    pub fn novo(token: impl Into<String>, inatividade: Duration) -> Arc<Self> {
        Arc::new(Self { token: token.into(), inatividade, estado: Mutex::new(Estado::default()) })
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// Sem sinal do dono por este tempo, a conexão é considerada morta e a vaga abre.
    pub fn inatividade(&self) -> Duration {
        self.inatividade
    }

    /// Compara em tempo constante, para o tempo de resposta não revelar o token.
    pub fn autorizado(&self, candidato: &str) -> bool {
        if self.token.is_empty() {
            return false; // sem token configurado, ninguém entra
        }
        let (a, b) = (self.token.as_bytes(), candidato.as_bytes());
        let mut diferenca = a.len() ^ b.len();
        for i in 0..a.len().max(b.len()) {
            diferenca |= (a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0)) as usize;
        }
        diferenca == 0
    }

    pub fn entrar(self: &Arc<Self>, cliente: &str) -> Entrada {
        let mut estado = self.estado.lock().unwrap();
        let mut substituiu = false;
        match &mut estado.dono {
            Some(dono) if dono.cliente != cliente => return Entrada::Ocupado,
            Some(dono) => {
                if let Some(derrubar) = dono.derrubar.take() {
                    let _ = derrubar.send(());
                }
                substituiu = true;
            }
            None => {}
        }
        estado.proxima_geracao += 1;
        let geracao = estado.proxima_geracao;
        let (derrubar, derrubada) = oneshot::channel();
        estado.dono = Some(Dono { cliente: cliente.to_string(), geracao, derrubar: Some(derrubar) });
        Entrada::Dono(Vaga { acesso: self.clone(), geracao, liberada: false, substituiu, derrubada })
    }
}

impl Vaga {
    /// Solta a vaga. Devolve true se esta conexão ainda era a dona (e não foi substituída).
    pub fn liberar(mut self) -> bool {
        self.soltar()
    }

    fn soltar(&mut self) -> bool {
        if self.liberada {
            return false;
        }
        self.liberada = true;
        let mut estado = self.acesso.estado.lock().unwrap();
        if estado.dono.as_ref().is_some_and(|d| d.geracao == self.geracao) {
            estado.dono = None;
            true
        } else {
            false
        }
    }
}

impl Drop for Vaga {
    fn drop(&mut self) {
        self.soltar();
    }
}

/// 128 bits aleatórios do sistema, em hexadecimal.
pub fn gerar_token() -> io::Result<String> {
    let mut bytes = [0u8; 16];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn token_valido(texto: &str) -> bool {
    texto.len() == 32 && texto.chars().all(|c| c.is_ascii_hexdigit())
}

/// Token salvo em `arquivo`. Reaproveitá-lo faz o atalho do celular continuar valendo a cada
/// reinício; `novo` (ou um arquivo inválido) gera outro. O arquivo é legível só pelo dono.
pub fn token_do_arquivo(arquivo: &Path, novo: bool) -> io::Result<String> {
    if !novo {
        if let Ok(texto) = fs::read_to_string(arquivo) {
            let texto = texto.trim();
            if token_valido(texto) {
                return Ok(texto.to_string());
            }
        }
    }
    let token = gerar_token()?;
    if let Some(pasta) = arquivo.parent() {
        fs::create_dir_all(pasta)?;
    }
    let mut f = fs::File::create(arquivo)?;
    f.set_permissions(fs::Permissions::from_mode(0o600))?;
    f.write_all(token.as_bytes())?;
    Ok(token)
}

/// `$XDG_CONFIG_HOME/vpad/token`, ou `~/.config/vpad/token`.
pub fn caminho_do_token(xdg: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    let base = match xdg.filter(|x| !x.is_empty()) {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from(home.filter(|h| !h.is_empty())?).join(".config"),
    };
    Some(base.join("vpad").join("token"))
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

    fn acesso() -> Arc<Acesso> {
        Acesso::novo("segredo", Duration::from_secs(6))
    }

    fn vaga(e: Entrada) -> Vaga {
        match e {
            Entrada::Dono(v) => v,
            Entrada::Ocupado => panic!("esperava assumir o controle, mas está ocupado"),
        }
    }

    #[test]
    fn so_o_token_exato_e_aceito() {
        let a = acesso();
        assert!(a.autorizado("segredo"));
        assert!(!a.autorizado("segredO"));
        assert!(!a.autorizado("segred"));
        assert!(!a.autorizado("segredo "));
        assert!(!a.autorizado(""));
    }

    #[test]
    fn token_vazio_nunca_autoriza_ninguem() {
        let a = Acesso::novo("", Duration::from_secs(6));
        assert!(!a.autorizado(""));
        assert!(!a.autorizado("qualquer"));
    }

    #[test]
    fn o_primeiro_assume_e_o_segundo_fica_de_fora() {
        let a = acesso();
        let _primeiro = vaga(a.entrar("celular-1"));
        assert!(matches!(a.entrar("celular-2"), Entrada::Ocupado));
        assert!(matches!(a.entrar("celular-3"), Entrada::Ocupado));
    }

    #[test]
    fn liberar_abre_a_vaga_para_o_proximo() {
        let a = acesso();
        let v = vaga(a.entrar("celular-1"));
        assert!(v.liberar(), "quem liberou era o dono");
        let _segundo = vaga(a.entrar("celular-2"));
    }

    #[test]
    fn largar_a_vaga_sem_liberar_tambem_abre() {
        let a = acesso();
        drop(vaga(a.entrar("celular-1")));
        let _segundo = vaga(a.entrar("celular-2"));
    }

    #[test]
    fn o_mesmo_aparelho_reassume_e_derruba_a_conexao_antiga() {
        let a = acesso();
        let mut antiga = vaga(a.entrar("celular-1"));
        let nova = vaga(a.entrar("celular-1"));

        assert!(nova.substituiu);
        assert!(!antiga.substituiu);
        assert!(antiga.derrubada.try_recv().is_ok(), "a conexão antiga precisa ser avisada");

        // a antiga acabar não pode soltar a vaga que já é da nova
        assert!(!antiga.liberar());
        assert!(matches!(a.entrar("celular-2"), Entrada::Ocupado));
        assert!(nova.liberar());
    }

    #[test]
    fn largar_a_conexao_antiga_nao_libera_a_nova() {
        let a = acesso();
        let antiga = vaga(a.entrar("celular-1"));
        let _nova = vaga(a.entrar("celular-1"));
        drop(antiga);
        assert!(matches!(a.entrar("celular-2"), Entrada::Ocupado));
    }

    #[test]
    fn expoe_o_tempo_de_inatividade() {
        assert_eq!(acesso().inatividade(), Duration::from_secs(6));
    }

    // ---- token em arquivo ----

    struct Pasta(PathBuf);
    impl Pasta {
        fn nova(nome: &str) -> Pasta {
            let p = std::env::temp_dir().join(format!("vpad-token-{}-{nome}", std::process::id()));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            Pasta(p)
        }
        fn token(&self) -> PathBuf {
            self.0.join("vpad").join("token")
        }
    }
    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn e_hex32(s: &str) -> bool {
        s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit())
    }

    #[test]
    fn gera_tokens_aleatorios_de_32_caracteres_hexadecimais() {
        let a = gerar_token().unwrap();
        let b = gerar_token().unwrap();
        assert!(e_hex32(&a), "{a}");
        assert!(e_hex32(&b), "{b}");
        assert_ne!(a, b);
    }

    #[test]
    fn cria_o_arquivo_do_token_so_para_o_dono() {
        let pasta = Pasta::nova("cria");
        let token = token_do_arquivo(&pasta.token(), false).unwrap();
        assert!(e_hex32(&token));
        assert_eq!(fs::read_to_string(pasta.token()).unwrap().trim(), token);
        let modo = fs::metadata(pasta.token()).unwrap().permissions().mode() & 0o777;
        assert_eq!(modo, 0o600, "o token não pode ser legível por outros usuários");
    }

    #[test]
    fn reaproveita_o_token_salvo_para_o_atalho_do_celular_continuar_valendo() {
        let pasta = Pasta::nova("reusa");
        let primeiro = token_do_arquivo(&pasta.token(), false).unwrap();
        let segundo = token_do_arquivo(&pasta.token(), false).unwrap();
        assert_eq!(primeiro, segundo);
    }

    #[test]
    fn pedir_um_token_novo_substitui_o_antigo() {
        let pasta = Pasta::nova("novo");
        let antigo = token_do_arquivo(&pasta.token(), false).unwrap();
        let novo = token_do_arquivo(&pasta.token(), true).unwrap();
        assert_ne!(antigo, novo);
        assert_eq!(token_do_arquivo(&pasta.token(), false).unwrap(), novo);
    }

    #[test]
    fn arquivo_invalido_vira_um_token_novo() {
        let pasta = Pasta::nova("invalido");
        fs::create_dir_all(pasta.token().parent().unwrap()).unwrap();
        for lixo in ["", "curto", "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"] {
            fs::write(pasta.token(), lixo).unwrap();
            let token = token_do_arquivo(&pasta.token(), false).unwrap();
            assert!(e_hex32(&token), "{lixo:?} -> {token}");
        }
    }

    #[test]
    fn caminho_do_token_segue_o_xdg() {
        assert_eq!(caminho_do_token(Some("/x"), Some("/h")), Some(PathBuf::from("/x/vpad/token")));
        assert_eq!(caminho_do_token(None, Some("/h")), Some(PathBuf::from("/h/.config/vpad/token")));
        assert_eq!(caminho_do_token(None, None), None);
    }
}
