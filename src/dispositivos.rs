//! Dispositivos pareados. Cada celular recebe um token só dele (128 bits aleatórios), e o
//! servidor guarda apenas o hash SHA-256: quem ler o arquivo não consegue usar os tokens, e
//! cada aparelho pode ser revogado sem afetar os outros.

use std::{
    fs,
    io::{self, Read, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::pareamento::iguais;

const NOME_PADRAO: &str = "dispositivo";
const NOME_MAXIMO: usize = 40;

/// O que um aparelho pareado pode fazer além de controlar jogos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissoes {
    /// usar o modo teclado, que digita no PC
    pub teclado: bool,
}

/// O que se pode mostrar de um dispositivo pareado (nunca o token nem o hash).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registro {
    pub id: String,
    pub nome: String,
    /// segundos desde 1970
    pub criado: u64,
    /// o aparelho pode usar o modo teclado (que digita no PC)
    pub teclado: bool,
}

/// Resultado de um pareamento: o token só existe aqui e no celular.
pub struct Pareado {
    pub id: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Guardado {
    id: String,
    nome: String,
    hash: String,
    criado: u64,
    /// arquivos das versões anteriores não têm o campo: valem como "sem permissão"
    #[serde(default)]
    teclado: bool,
}

pub struct Dispositivos {
    arquivo: Option<PathBuf>,
    lista: Mutex<Vec<Guardado>>,
}

fn aleatorio_hex(bytes: usize) -> io::Result<String> {
    let mut b = vec![0u8; bytes];
    fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// 128 bits aleatórios do sistema, em hexadecimal.
pub fn gerar_token() -> io::Result<String> {
    aleatorio_hex(16)
}

pub fn gerar_id() -> io::Result<String> {
    aleatorio_hex(4)
}

pub(crate) fn hash_do_token(token: &str) -> String {
    Sha256::digest(token.as_bytes()).iter().map(|x| format!("{x:02x}")).collect()
}

/// Lê o cadastro; arquivo inexistente é um cadastro vazio.
fn ler(arquivo: &Path) -> Result<Vec<Guardado>, String> {
    match fs::read_to_string(arquivo) {
        Ok(texto) => serde_json::from_str(&texto).map_err(|e| e.to_string()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(e.to_string()),
    }
}

/// Nome sem caracteres de controle (que poderiam mexer no terminal), com até 40 caracteres.
pub fn limpar_nome(nome: &str) -> String {
    let limpo: String = nome.chars().filter(|c| !c.is_control()).collect();
    let limpo: String = limpo.trim().chars().take(NOME_MAXIMO).collect();
    let limpo = limpo.trim_end().to_string();
    if limpo.is_empty() { NOME_PADRAO.to_string() } else { limpo }
}

impl Dispositivos {
    pub fn em_memoria() -> Self {
        Self { arquivo: None, lista: Mutex::new(vec![]) }
    }

    /// Carrega o arquivo; se não existir ou estiver inválido, começa vazio (ninguém pareado).
    pub fn com_arquivo(arquivo: PathBuf) -> Self {
        let lista = ler(&arquivo).unwrap_or_else(|e| {
            eprintln!("ignorando {}: {e}", arquivo.display());
            vec![]
        });
        Self { arquivo: Some(arquivo), lista: Mutex::new(lista) }
    }

    /// Relê o arquivo, que outro processo (o comando `--revogar`) pode ter mudado. Se o
    /// arquivo estiver ilegível naquele instante, mantém o que já se sabia.
    pub fn recarregar(&self) {
        let Some(arquivo) = &self.arquivo else { return };
        if let Ok(lista) = ler(arquivo) {
            *self.lista.lock().unwrap() = lista;
        }
    }

    /// Cadastra um aparelho novo, sem permissão de teclado.
    pub fn parear(&self, nome: &str, agora: u64) -> io::Result<Pareado> {
        self.parear_com(nome, agora, false)
    }

    /// Cadastra um aparelho novo, já com (ou sem) a permissão do modo teclado. Só vale se o
    /// arquivo for gravado com sucesso.
    pub fn parear_com(&self, nome: &str, agora: u64, teclado: bool) -> io::Result<Pareado> {
        self.recarregar(); // não desfazer o que outro processo acabou de revogar
        let token = gerar_token()?;
        let id = gerar_id()?;
        let novo = Guardado { id: id.clone(), nome: limpar_nome(nome), hash: hash_do_token(&token), criado: agora, teclado };

        let mut lista = self.lista.lock().unwrap();
        let mut proxima = lista.clone();
        proxima.push(novo);
        self.gravar(&proxima)?;
        *lista = proxima;
        Ok(Pareado { id, token })
    }

    /// O token pertence a algum aparelho pareado? Compara os hashes em tempo constante.
    pub fn autorizado(&self, token: &str) -> bool {
        let candidato = hash_do_token(token);
        let lista = self.lista.lock().unwrap();
        // percorre todos, sem parar no primeiro, para o tempo não revelar a posição
        lista.iter().fold(false, |achou, g| iguais(&g.hash, &candidato) | achou)
    }

    /// Uma consulta só: o token é de um aparelho pareado? Se for, quais as permissões dele.
    pub fn consultar(&self, token: &str) -> Option<Permissoes> {
        self.identificar(token).map(|r| Permissoes { teclado: r.teclado })
    }

    /// O aparelho dono deste token (id, nome, permissões), se for de um aparelho pareado.
    pub fn identificar(&self, token: &str) -> Option<Registro> {
        let candidato = hash_do_token(token);
        let lista = self.lista.lock().unwrap();
        // percorre todos, sem parar no primeiro, para o tempo não revelar a posição
        lista.iter().fold(None, |achado, g| {
            if iguais(&g.hash, &candidato) {
                Some(Registro { id: g.id.clone(), nome: g.nome.clone(), criado: g.criado, teclado: g.teclado })
            } else {
                achado
            }
        })
    }

    /// O aparelho dono deste token pode usar o modo teclado? Token desconhecido: não.
    pub fn teclado_permitido(&self, token: &str) -> bool {
        let candidato = hash_do_token(token);
        let lista = self.lista.lock().unwrap();
        lista.iter().fold(false, |permitido, g| permitido | (iguais(&g.hash, &candidato) & g.teclado))
    }

    /// Libera ou tira o modo teclado do aparelho `id`. Devolve false se o id não existe.
    pub fn definir_teclado(&self, id: &str, permitir: bool) -> io::Result<bool> {
        self.recarregar();
        let mut lista = self.lista.lock().unwrap();
        let Some(posicao) = lista.iter().position(|g| g.id == id) else { return Ok(false) };
        let mut proxima = lista.clone();
        proxima[posicao].teclado = permitir;
        self.gravar(&proxima)?;
        *lista = proxima;
        Ok(true)
    }

    pub fn listar(&self) -> Vec<Registro> {
        let lista = self.lista.lock().unwrap();
        lista
            .iter()
            .map(|g| Registro { id: g.id.clone(), nome: g.nome.clone(), criado: g.criado, teclado: g.teclado })
            .collect()
    }

    /// Nome cadastrado do aparelho `id` (para mensagens no terminal).
    pub fn nome_de(&self, id: &str) -> String {
        let lista = self.lista.lock().unwrap();
        lista.iter().find(|g| g.id == id).map(|g| g.nome.clone()).unwrap_or_default()
    }

    /// Devolve false se não havia aparelho com esse id.
    pub fn revogar(&self, id: &str) -> io::Result<bool> {
        self.recarregar();
        let mut lista = self.lista.lock().unwrap();
        let proxima: Vec<_> = lista.iter().filter(|g| g.id != id).cloned().collect();
        if proxima.len() == lista.len() {
            return Ok(false);
        }
        self.gravar(&proxima)?;
        *lista = proxima;
        Ok(true)
    }

    /// Revoga todos; devolve quantos eram.
    pub fn revogar_todos(&self) -> io::Result<usize> {
        self.recarregar();
        let mut lista = self.lista.lock().unwrap();
        let quantos = lista.len();
        self.gravar(&[])?;
        lista.clear();
        Ok(quantos)
    }

    /// Escreve num arquivo temporário (só legível pelo dono) e renomeia por cima do atual.
    fn gravar(&self, lista: &[Guardado]) -> io::Result<()> {
        let Some(arquivo) = &self.arquivo else { return Ok(()) };
        if let Some(pasta) = arquivo.parent() {
            fs::create_dir_all(pasta)?;
        }
        let texto = serde_json::to_string_pretty(lista).map_err(io::Error::other)?;
        let temporario = arquivo.with_extension("json.tmp");
        let mut f = fs::File::create(&temporario)?;
        f.set_permissions(fs::Permissions::from_mode(0o600))?;
        f.write_all(texto.as_bytes())?;
        drop(f);
        fs::rename(&temporario, arquivo)
    }
}

/// `$XDG_CONFIG_HOME/vpad/dispositivos.json`, ou `~/.config/vpad/dispositivos.json`.
pub fn caminho_dos_dispositivos(xdg: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    let base = match xdg.filter(|x| !x.is_empty()) {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from(home.filter(|h| !h.is_empty())?).join(".config"),
    };
    Some(base.join("vpad").join("dispositivos.json"))
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    struct Pasta(PathBuf);
    impl Pasta {
        fn nova(nome: &str) -> Pasta {
            let p = std::env::temp_dir().join(format!("vpad-disp-{}-{nome}", std::process::id()));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            Pasta(p)
        }
        fn arquivo(&self) -> PathBuf {
            self.0.join("vpad").join("dispositivos.json")
        }
    }
    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn hex(s: &str, n: usize) -> bool {
        s.len() == n && s.chars().all(|c| c.is_ascii_hexdigit())
    }

    #[test]
    fn tokens_e_ids_sao_aleatorios_e_do_tamanho_certo() {
        let (a, b) = (gerar_token().unwrap(), gerar_token().unwrap());
        assert!(hex(&a, 32) && hex(&b, 32));
        assert_ne!(a, b);
        assert!(hex(&gerar_id().unwrap(), 8));
    }

    #[test]
    fn parear_entrega_um_token_que_passa_a_valer() {
        let d = Dispositivos::em_memoria();
        let p = d.parear("iPhone", 1000).unwrap();
        assert!(hex(&p.token, 32) && hex(&p.id, 8));
        assert!(d.autorizado(&p.token));
    }

    #[test]
    fn so_o_token_exato_vale() {
        let d = Dispositivos::em_memoria();
        let p = d.parear("iPhone", 1000).unwrap();
        for ruim in ["", "errado", &p.token[..31], &format!("{}0", p.token), &p.token.to_uppercase()] {
            assert!(!d.autorizado(ruim), "{ruim:?}");
        }
        assert!(!Dispositivos::em_memoria().autorizado(&p.token), "sem ninguém pareado nada vale");
    }

    #[test]
    fn cada_dispositivo_tem_o_seu_token() {
        let d = Dispositivos::em_memoria();
        let a = d.parear("iPhone", 1000).unwrap();
        let b = d.parear("iPad", 1001).unwrap();
        assert_ne!(a.token, b.token);
        assert_ne!(a.id, b.id);
        assert!(d.autorizado(&a.token) && d.autorizado(&b.token));
    }

    #[test]
    fn revogar_um_nao_afeta_os_outros() {
        let d = Dispositivos::em_memoria();
        let a = d.parear("iPhone", 1000).unwrap();
        let b = d.parear("iPad", 1001).unwrap();
        assert!(d.revogar(&a.id).unwrap());
        assert!(!d.autorizado(&a.token));
        assert!(d.autorizado(&b.token));
        assert!(!d.revogar(&a.id).unwrap(), "já revogado");
        assert!(!d.revogar("naoexiste").unwrap());
    }

    #[test]
    fn revogar_todos_derruba_todos() {
        let d = Dispositivos::em_memoria();
        let tokens: Vec<_> = (0..3).map(|i| d.parear("x", i).unwrap().token).collect();
        assert_eq!(d.revogar_todos().unwrap(), 3);
        assert!(tokens.iter().all(|t| !d.autorizado(t)));
        assert!(d.listar().is_empty());
    }

    #[test]
    fn listar_mostra_id_nome_e_data_sem_revelar_o_token() {
        let d = Dispositivos::em_memoria();
        let p = d.parear("iPhone", 1234).unwrap();
        let lista = d.listar();
        assert_eq!(lista.len(), 1);
        assert_eq!((lista[0].id.as_str(), lista[0].nome.as_str(), lista[0].criado), (p.id.as_str(), "iPhone", 1234));
        assert!(!format!("{lista:?}").contains(&p.token));
    }

    #[test]
    fn o_nome_e_limpo_e_limitado() {
        let d = Dispositivos::em_memoria();
        for (entrada, esperado) in [
            ("  iPhone do Wilian  ", "iPhone do Wilian"),
            ("", "dispositivo"),
            ("   ", "dispositivo"),
            ("a\nb\tc\u{7}", "abc"),
            (&"x".repeat(100), &"x".repeat(40)),
        ] {
            d.parear(entrada, 1).unwrap();
            assert_eq!(d.listar().last().unwrap().nome, esperado, "{entrada:?}");
        }
    }

    #[test]
    fn grava_so_o_hash_com_permissao_restrita_e_recarrega() {
        let pasta = Pasta::nova("grava");
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        let p = d.parear("iPhone", 1000).unwrap();

        let texto = fs::read_to_string(pasta.arquivo()).unwrap();
        assert!(!texto.contains(&p.token), "o token nunca pode ir para o disco");
        assert!(texto.contains(&hash_do_token(&p.token)));
        let modo = fs::metadata(pasta.arquivo()).unwrap().permissions().mode() & 0o777;
        assert_eq!(modo, 0o600);

        let recarregado = Dispositivos::com_arquivo(pasta.arquivo());
        assert!(recarregado.autorizado(&p.token), "o aparelho continua pareado depois de reiniciar");
        assert_eq!(recarregado.listar().len(), 1);
    }

    #[test]
    fn revogar_tambem_vale_depois_de_reiniciar() {
        let pasta = Pasta::nova("revoga");
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        let p = d.parear("iPhone", 1000).unwrap();
        d.revogar(&p.id).unwrap();
        assert!(!Dispositivos::com_arquivo(pasta.arquivo()).autorizado(&p.token));
    }

    #[test]
    fn arquivo_corrompido_comeca_vazio_sem_quebrar() {
        let pasta = Pasta::nova("lixo");
        fs::create_dir_all(pasta.arquivo().parent().unwrap()).unwrap();
        fs::write(pasta.arquivo(), "isto nao e json").unwrap();
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        assert!(d.listar().is_empty());
        assert!(!d.autorizado("qualquer"));
    }

    #[test]
    fn falha_ao_gravar_devolve_erro_e_nao_pareia() {
        let pasta = Pasta::nova("falha");
        fs::write(pasta.0.join("vpad"), "x").unwrap(); // "vpad" é arquivo: não dá para criar dentro
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        assert!(d.parear("iPhone", 1000).is_err());
        assert!(d.listar().is_empty());
    }

    #[test]
    fn caminho_dos_dispositivos_segue_o_xdg() {
        assert_eq!(
            caminho_dos_dispositivos(Some("/x"), Some("/h")),
            Some(PathBuf::from("/x/vpad/dispositivos.json"))
        );
        assert_eq!(
            caminho_dos_dispositivos(None, Some("/h")),
            Some(PathBuf::from("/h/.config/vpad/dispositivos.json"))
        );
        assert_eq!(caminho_dos_dispositivos(None, None), None);
    }

    // ---- vários processos no mesmo arquivo (servidor rodando + comando --revogar) ----

    #[test]
    fn revogar_por_outro_processo_vale_no_servidor_depois_de_recarregar() {
        let pasta = Pasta::nova("recarrega-revoga");
        let servidor = Dispositivos::com_arquivo(pasta.arquivo());
        let p = servidor.parear("iPhone", 1000).unwrap();

        let comando = Dispositivos::com_arquivo(pasta.arquivo());
        assert!(comando.revogar(&p.id).unwrap());

        assert!(servidor.autorizado(&p.token), "ainda não leu o arquivo");
        servidor.recarregar();
        assert!(!servidor.autorizado(&p.token));
    }

    #[test]
    fn aparelho_pareado_por_outro_processo_aparece_depois_de_recarregar() {
        let pasta = Pasta::nova("recarrega-novo");
        let servidor = Dispositivos::com_arquivo(pasta.arquivo());
        let outro = Dispositivos::com_arquivo(pasta.arquivo());
        let p = outro.parear("iPad", 1000).unwrap();
        servidor.recarregar();
        assert!(servidor.autorizado(&p.token));
    }

    #[test]
    fn parear_nao_ressuscita_um_aparelho_revogado_por_outro_processo() {
        let pasta = Pasta::nova("sem-ressuscitar");
        let servidor = Dispositivos::com_arquivo(pasta.arquivo());
        let velho = servidor.parear("iPhone", 1000).unwrap();

        Dispositivos::com_arquivo(pasta.arquivo()).revogar(&velho.id).unwrap();

        // o servidor ainda não recarregou, mas ao gravar um aparelho novo não pode trazer o velho de volta
        let novo = servidor.parear("iPad", 1001).unwrap();
        let depois = Dispositivos::com_arquivo(pasta.arquivo());
        assert!(!depois.autorizado(&velho.token));
        assert!(depois.autorizado(&novo.token));
    }

    #[test]
    fn recarregar_sem_mudanca_nao_perde_nada() {
        let pasta = Pasta::nova("recarrega-igual");
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        let p = d.parear("iPhone", 1000).unwrap();
        d.recarregar();
        d.recarregar();
        assert!(d.autorizado(&p.token));
    }

    #[test]
    fn recarregar_em_memoria_nao_faz_nada() {
        let d = Dispositivos::em_memoria();
        let p = d.parear("iPhone", 1000).unwrap();
        d.recarregar();
        assert!(d.autorizado(&p.token));
    }

    #[test]
    fn arquivo_apagado_por_fora_deixa_ninguem_pareado() {
        let pasta = Pasta::nova("apagado");
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        let p = d.parear("iPhone", 1000).unwrap();
        fs::remove_file(pasta.arquivo()).unwrap();
        d.recarregar();
        assert!(!d.autorizado(&p.token));
    }

    // ---------- permissão do modo teclado, por aparelho ----------

    #[test]
    fn aparelho_novo_nasce_sem_permissao_de_teclado() {
        let d = Dispositivos::em_memoria();
        let p = d.parear("iPhone", 1000).unwrap();
        assert!(!d.teclado_permitido(&p.token));
        assert!(!d.listar()[0].teclado);
    }

    #[test]
    fn pode_nascer_com_a_permissao_se_o_dono_do_pc_aprovou_assim() {
        let d = Dispositivos::em_memoria();
        let p = d.parear_com("iPhone", 1000, true).unwrap();
        assert!(d.teclado_permitido(&p.token));
        assert!(d.listar()[0].teclado);
    }

    #[test]
    fn a_permissao_e_so_do_aparelho_que_a_recebeu() {
        let d = Dispositivos::em_memoria();
        let a = d.parear_com("iPhone", 1000, true).unwrap();
        let b = d.parear("iPad", 1001).unwrap();
        assert!(d.teclado_permitido(&a.token));
        assert!(!d.teclado_permitido(&b.token));
    }

    #[test]
    fn token_desconhecido_ou_vazio_nunca_tem_permissao() {
        let d = Dispositivos::em_memoria();
        d.parear_com("iPhone", 1000, true).unwrap();
        for ruim in ["", "errado", "0123456789abcdef0123456789abcdef"] {
            assert!(!d.teclado_permitido(ruim), "{ruim:?}");
        }
    }

    #[test]
    fn permitir_e_negar_depois_de_pareado() {
        let d = Dispositivos::em_memoria();
        let p = d.parear("iPhone", 1000).unwrap();
        assert!(d.definir_teclado(&p.id, true).unwrap());
        assert!(d.teclado_permitido(&p.token));
        assert!(d.definir_teclado(&p.id, false).unwrap());
        assert!(!d.teclado_permitido(&p.token));
        assert!(!d.definir_teclado("naoexiste", true).unwrap(), "id desconhecido");
    }

    #[test]
    fn a_permissao_sobrevive_ao_reinicio_e_arquivos_antigos_valem_como_negado() {
        let pasta = Pasta::nova("perm-teclado");
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        let p = d.parear_com("iPhone", 1000, true).unwrap();
        assert!(Dispositivos::com_arquivo(pasta.arquivo()).teclado_permitido(&p.token));

        // arquivo das versões anteriores: sem o campo `teclado`
        let antigo = Pasta::nova("perm-antigo");
        fs::create_dir_all(antigo.arquivo().parent().unwrap()).unwrap();
        let token = "0123456789abcdef0123456789abcdef";
        fs::write(
            antigo.arquivo(),
            format!(r#"[{{"id":"ab12cd34","nome":"iPhone","hash":"{}","criado":1}}]"#, hash_do_token(token)),
        )
        .unwrap();
        let d = Dispositivos::com_arquivo(antigo.arquivo());
        assert!(d.autorizado(token), "continua pareado");
        assert!(!d.teclado_permitido(token), "mas sem teclado até o dono do PC liberar");
    }

    #[test]
    fn mudar_a_permissao_por_outro_processo_vale_depois_de_recarregar() {
        let pasta = Pasta::nova("perm-recarrega");
        let servidor = Dispositivos::com_arquivo(pasta.arquivo());
        let p = servidor.parear_com("iPhone", 1000, true).unwrap();

        Dispositivos::com_arquivo(pasta.arquivo()).definir_teclado(&p.id, false).unwrap();

        assert!(servidor.teclado_permitido(&p.token), "ainda não leu o arquivo");
        servidor.recarregar();
        assert!(!servidor.teclado_permitido(&p.token));
    }

    #[test]
    fn revogar_o_aparelho_tira_tambem_a_permissao() {
        let d = Dispositivos::em_memoria();
        let p = d.parear_com("iPhone", 1000, true).unwrap();
        d.revogar(&p.id).unwrap();
        assert!(!d.teclado_permitido(&p.token));
    }

    #[test]
    fn falha_ao_gravar_a_permissao_devolve_erro_e_nao_muda_nada() {
        let pasta = Pasta::nova("perm-falha");
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        let p = d.parear("iPhone", 1000).unwrap();
        // depois de pareado, a pasta vira um arquivo comum: não dá mais para gravar
        fs::remove_dir_all(pasta.arquivo().parent().unwrap()).unwrap();
        fs::write(pasta.arquivo().parent().unwrap(), "x").unwrap();
        assert!(d.definir_teclado(&p.id, true).is_err());
        assert!(!d.teclado_permitido(&p.token));
    }

    // ---------- consulta única (autorizado + permissões) ----------

    #[test]
    fn consultar_devolve_as_permissoes_do_aparelho_ou_nada_se_nao_for_pareado() {
        let d = Dispositivos::em_memoria();
        let sem = d.parear("iPad", 1000).unwrap();
        let com = d.parear_com("iPhone", 1001, true).unwrap();
        assert_eq!(d.consultar(&sem.token), Some(Permissoes { teclado: false }));
        assert_eq!(d.consultar(&com.token), Some(Permissoes { teclado: true }));
        assert_eq!(d.consultar("errado"), None);
        assert_eq!(d.consultar(""), None);
    }

    #[test]
    fn consultar_acompanha_a_revogacao_e_a_mudanca_de_permissao() {
        let d = Dispositivos::em_memoria();
        let p = d.parear_com("iPhone", 1000, true).unwrap();
        d.definir_teclado(&p.id, false).unwrap();
        assert_eq!(d.consultar(&p.token), Some(Permissoes { teclado: false }));
        d.revogar(&p.id).unwrap();
        assert_eq!(d.consultar(&p.token), None);
    }

    #[test]
    fn identificar_devolve_id_e_nome_so_para_tokens_pareados() {
        let d = Dispositivos::em_memoria();
        let p = d.parear_com("iPhone do Wilian", 1000, true).unwrap();
        let r = d.identificar(&p.token).unwrap();
        assert_eq!((r.id.as_str(), r.nome.as_str(), r.teclado), (p.id.as_str(), "iPhone do Wilian", true));
        assert!(d.identificar("errado").is_none());
    }
}
