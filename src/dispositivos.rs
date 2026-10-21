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
/// Quanto tempo um token vale depois de o aparelho ser pareado (90 dias).
pub const VALIDADE_DO_TOKEN: u64 = 90 * 86_400;
/// O "visto por último" é gravado no disco no máximo uma vez por minuto, para não escrever
/// o arquivo a cada mensagem do WebSocket.
const INTERVALO_DO_ULTIMO_VISTO: u64 = 60;

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
    /// segundos desde 1970 em que o token deixa de valer
    pub expira: u64,
    /// último instante em que o token foi usado, se alguma vez foi
    pub ultimo_visto: Option<u64>,
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
    /// ausente nos arquivos antigos; `migrar` preenche a partir de `criado`
    #[serde(default)]
    expira: Option<u64>,
    /// não vai para o cadastro principal (que o `--revogar` reescreve): fica num arquivo à
    /// parte, para anotar o último uso sem nunca ressuscitar um aparelho revogado
    #[serde(default, skip_serializing)]
    visto: Option<u64>,
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

/// Agora, em segundos desde 1970.
pub fn agora() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Preenche a validade dos cadastros antigos, que não tinham o campo: a contagem começa na
/// data em que o aparelho foi pareado.
fn migrar(mut lista: Vec<Guardado>) -> Vec<Guardado> {
    for g in &mut lista {
        if g.expira.is_none() {
            g.expira = Some(g.criado.saturating_add(VALIDADE_DO_TOKEN));
        }
    }
    lista
}

fn expira_de(g: &Guardado) -> u64 {
    g.expira.unwrap_or_else(|| g.criado.saturating_add(VALIDADE_DO_TOKEN))
}

/// Lê o cadastro; arquivo inexistente é um cadastro vazio.
fn ler(arquivo: &Path) -> Result<Vec<Guardado>, String> {
    match fs::read_to_string(arquivo) {
        Ok(texto) => serde_json::from_str(&texto).map(migrar).map_err(|e| e.to_string()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(e.to_string()),
    }
}

/// O "visto por último" fica num arquivo à parte para que anotá-lo nunca reescreva o
/// cadastro (e não possa desfazer uma revogação feita por outro processo).
fn caminho_dos_vistos(arquivo: &Path) -> PathBuf {
    arquivo.with_extension("vistos.json")
}

fn ler_vistos(arquivo: &Path) -> std::collections::HashMap<String, u64> {
    fs::read_to_string(caminho_dos_vistos(arquivo))
        .ok()
        .and_then(|texto| serde_json::from_str(&texto).ok())
        .unwrap_or_default()
}

fn gravar_vistos(arquivo: &Path, lista: &[Guardado]) -> io::Result<()> {
    let vistos: std::collections::BTreeMap<&str, u64> =
        lista.iter().filter_map(|g| g.visto.map(|v| (g.id.as_str(), v))).collect();
    let destino = caminho_dos_vistos(arquivo);
    if vistos.is_empty() {
        let _ = fs::remove_file(destino);
        return Ok(());
    }
    let temporario = destino.with_extension("tmp");
    let mut f = fs::File::create(&temporario)?;
    f.set_permissions(fs::Permissions::from_mode(0o600))?;
    f.write_all(serde_json::to_string(&vistos).map_err(io::Error::other)?.as_bytes())?;
    drop(f);
    fs::rename(&temporario, destino)
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
        let mut lista = ler(&arquivo).unwrap_or_else(|e| {
            eprintln!("ignorando {}: {e}", arquivo.display());
            vec![]
        });
        let vistos = ler_vistos(&arquivo);
        for g in &mut lista {
            if let Some(&visto) = vistos.get(&g.id) {
                g.visto = Some(visto);
            }
        }
        Self { arquivo: Some(arquivo), lista: Mutex::new(lista) }
    }

    /// Relê o arquivo, que outro processo (o comando `--revogar`) pode ter mudado. Se o
    /// arquivo estiver ilegível naquele instante, mantém o que já se sabia. O "visto por
    /// último" mais recente (em memória ou no disco) é preservado.
    pub fn recarregar(&self) {
        let Some(arquivo) = &self.arquivo else { return };
        if let Ok(mut lista) = ler(arquivo) {
            let vistos = ler_vistos(arquivo);
            let mut atual = self.lista.lock().unwrap();
            for g in &mut lista {
                if let Some(&visto) = vistos.get(&g.id) {
                    g.visto = Some(visto);
                }
                if let Some(visto) = atual.iter().find(|a| a.id == g.id).and_then(|a| a.visto) {
                    if g.visto.is_none_or(|gravado| visto > gravado) {
                        g.visto = Some(visto);
                    }
                }
            }
            *atual = lista;
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
        let novo = Guardado {
            id: id.clone(),
            nome: limpar_nome(nome),
            hash: hash_do_token(&token),
            criado: agora,
            teclado,
            expira: Some(agora.saturating_add(VALIDADE_DO_TOKEN)),
            visto: None,
        };

        let mut lista = self.lista.lock().unwrap();
        let mut proxima = lista.clone();
        proxima.push(novo);
        self.gravar(&proxima)?;
        *lista = proxima;
        Ok(Pareado { id, token })
    }

    /// O token pertence a algum aparelho pareado e ainda não venceu? Compara os hashes em
    /// tempo constante e registra o "visto por último".
    pub fn autorizado(&self, token: &str, agora: u64) -> bool {
        self.encontrar(token, agora).is_some()
    }

    /// Uma consulta só: o token é de um aparelho pareado (e válido)? Se for, quais as
    /// permissões dele.
    pub fn consultar(&self, token: &str, agora: u64) -> Option<Permissoes> {
        self.encontrar(token, agora).map(|r| Permissoes { teclado: r.teclado })
    }

    /// O aparelho dono deste token (id, nome, permissões, validade e último uso), se o token
    /// for de um aparelho pareado e ainda não tiver vencido.
    pub fn identificar(&self, token: &str, agora: u64) -> Option<Registro> {
        self.encontrar(token, agora)
    }

    /// O aparelho dono deste token pode usar o modo teclado? Token desconhecido ou vencido: não.
    pub fn teclado_permitido(&self, token: &str, agora: u64) -> bool {
        self.encontrar(token, agora).is_some_and(|r| r.teclado)
    }

    /// Procura o token em toda a lista (sem parar no primeiro, para o tempo não revelar a
    /// posição) e devolve o registro se ele ainda não venceu. Aproveita para anotar o
    /// "visto por último", gravando no disco no máximo uma vez por minuto.
    fn encontrar(&self, token: &str, agora: u64) -> Option<Registro> {
        let candidato = hash_do_token(token);
        let mut lista = self.lista.lock().unwrap();
        let mut achado = None;
        let mut anotou = false;
        for g in lista.iter_mut() {
            if !iguais(&g.hash, &candidato) {
                continue;
            }
            let expira = expira_de(g);
            if agora >= expira {
                continue;
            }
            if g.visto.is_none_or(|visto| agora.saturating_sub(visto) >= INTERVALO_DO_ULTIMO_VISTO) {
                g.visto = Some(agora);
                anotou = true;
            }
            achado = Some(Registro {
                id: g.id.clone(),
                nome: g.nome.clone(),
                criado: g.criado,
                teclado: g.teclado,
                expira,
                ultimo_visto: g.visto,
            });
        }
        let para_gravar = anotou.then(|| lista.clone());
        drop(lista);
        if let Some(lista) = para_gravar {
            if let Some(arquivo) = &self.arquivo {
                let _ = gravar_vistos(arquivo, &lista);
            }
        }
        achado
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
            .map(|g| Registro {
                id: g.id.clone(),
                nome: g.nome.clone(),
                criado: g.criado,
                teclado: g.teclado,
                expira: expira_de(g),
                ultimo_visto: g.visto,
            })
            .collect()
    }

    /// O arquivo em que o cadastro é gravado (para vigiar mudanças feitas por outro processo).
    pub fn caminho(&self) -> Option<&Path> {
        self.arquivo.as_deref()
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

    /// Instante fixo usado pelas consultas: basta caber na validade dos tokens de teste.
    const AGORA: u64 = 10_000;

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
        assert!(d.autorizado(&p.token, AGORA));
    }

    #[test]
    fn so_o_token_exato_vale() {
        let d = Dispositivos::em_memoria();
        let p = d.parear("iPhone", 1000).unwrap();
        for ruim in ["", "errado", &p.token[..31], &format!("{}0", p.token), &p.token.to_uppercase()] {
            assert!(!d.autorizado(ruim, AGORA), "{ruim:?}");
        }
        assert!(!Dispositivos::em_memoria().autorizado(&p.token, AGORA), "sem ninguém pareado nada vale");
    }

    #[test]
    fn cada_dispositivo_tem_o_seu_token() {
        let d = Dispositivos::em_memoria();
        let a = d.parear("iPhone", 1000).unwrap();
        let b = d.parear("iPad", 1001).unwrap();
        assert_ne!(a.token, b.token);
        assert_ne!(a.id, b.id);
        assert!(d.autorizado(&a.token, AGORA) && d.autorizado(&b.token, AGORA));
    }

    #[test]
    fn revogar_um_nao_afeta_os_outros() {
        let d = Dispositivos::em_memoria();
        let a = d.parear("iPhone", 1000).unwrap();
        let b = d.parear("iPad", 1001).unwrap();
        assert!(d.revogar(&a.id).unwrap());
        assert!(!d.autorizado(&a.token, AGORA));
        assert!(d.autorizado(&b.token, AGORA));
        assert!(!d.revogar(&a.id).unwrap(), "já revogado");
        assert!(!d.revogar("naoexiste").unwrap());
    }

    #[test]
    fn revogar_todos_derruba_todos() {
        let d = Dispositivos::em_memoria();
        let tokens: Vec<_> = (0..3).map(|i| d.parear("x", i).unwrap().token).collect();
        assert_eq!(d.revogar_todos().unwrap(), 3);
        assert!(tokens.iter().all(|t| !d.autorizado(t, AGORA)));
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
        assert!(recarregado.autorizado(&p.token, AGORA), "o aparelho continua pareado depois de reiniciar");
        assert_eq!(recarregado.listar().len(), 1);
    }

    #[test]
    fn revogar_tambem_vale_depois_de_reiniciar() {
        let pasta = Pasta::nova("revoga");
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        let p = d.parear("iPhone", 1000).unwrap();
        d.revogar(&p.id).unwrap();
        assert!(!Dispositivos::com_arquivo(pasta.arquivo()).autorizado(&p.token, AGORA));
    }

    #[test]
    fn arquivo_corrompido_comeca_vazio_sem_quebrar() {
        let pasta = Pasta::nova("lixo");
        fs::create_dir_all(pasta.arquivo().parent().unwrap()).unwrap();
        fs::write(pasta.arquivo(), "isto nao e json").unwrap();
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        assert!(d.listar().is_empty());
        assert!(!d.autorizado("qualquer", AGORA));
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

        assert!(servidor.autorizado(&p.token, AGORA), "ainda não leu o arquivo");
        servidor.recarregar();
        assert!(!servidor.autorizado(&p.token, AGORA));
    }

    #[test]
    fn aparelho_pareado_por_outro_processo_aparece_depois_de_recarregar() {
        let pasta = Pasta::nova("recarrega-novo");
        let servidor = Dispositivos::com_arquivo(pasta.arquivo());
        let outro = Dispositivos::com_arquivo(pasta.arquivo());
        let p = outro.parear("iPad", 1000).unwrap();
        servidor.recarregar();
        assert!(servidor.autorizado(&p.token, AGORA));
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
        assert!(!depois.autorizado(&velho.token, AGORA));
        assert!(depois.autorizado(&novo.token, AGORA));
    }

    #[test]
    fn recarregar_sem_mudanca_nao_perde_nada() {
        let pasta = Pasta::nova("recarrega-igual");
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        let p = d.parear("iPhone", 1000).unwrap();
        d.recarregar();
        d.recarregar();
        assert!(d.autorizado(&p.token, AGORA));
    }

    #[test]
    fn recarregar_em_memoria_nao_faz_nada() {
        let d = Dispositivos::em_memoria();
        let p = d.parear("iPhone", 1000).unwrap();
        d.recarregar();
        assert!(d.autorizado(&p.token, AGORA));
    }

    #[test]
    fn arquivo_apagado_por_fora_deixa_ninguem_pareado() {
        let pasta = Pasta::nova("apagado");
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        let p = d.parear("iPhone", 1000).unwrap();
        fs::remove_file(pasta.arquivo()).unwrap();
        d.recarregar();
        assert!(!d.autorizado(&p.token, AGORA));
    }

    // ---------- permissão do modo teclado, por aparelho ----------

    #[test]
    fn aparelho_novo_nasce_sem_permissao_de_teclado() {
        let d = Dispositivos::em_memoria();
        let p = d.parear("iPhone", 1000).unwrap();
        assert!(!d.teclado_permitido(&p.token, AGORA));
        assert!(!d.listar()[0].teclado);
    }

    #[test]
    fn pode_nascer_com_a_permissao_se_o_dono_do_pc_aprovou_assim() {
        let d = Dispositivos::em_memoria();
        let p = d.parear_com("iPhone", 1000, true).unwrap();
        assert!(d.teclado_permitido(&p.token, AGORA));
        assert!(d.listar()[0].teclado);
    }

    #[test]
    fn a_permissao_e_so_do_aparelho_que_a_recebeu() {
        let d = Dispositivos::em_memoria();
        let a = d.parear_com("iPhone", 1000, true).unwrap();
        let b = d.parear("iPad", 1001).unwrap();
        assert!(d.teclado_permitido(&a.token, AGORA));
        assert!(!d.teclado_permitido(&b.token, AGORA));
    }

    #[test]
    fn token_desconhecido_ou_vazio_nunca_tem_permissao() {
        let d = Dispositivos::em_memoria();
        d.parear_com("iPhone", 1000, true).unwrap();
        for ruim in ["", "errado", "0123456789abcdef0123456789abcdef"] {
            assert!(!d.teclado_permitido(ruim, AGORA), "{ruim:?}");
        }
    }

    #[test]
    fn permitir_e_negar_depois_de_pareado() {
        let d = Dispositivos::em_memoria();
        let p = d.parear("iPhone", 1000).unwrap();
        assert!(d.definir_teclado(&p.id, true).unwrap());
        assert!(d.teclado_permitido(&p.token, AGORA));
        assert!(d.definir_teclado(&p.id, false).unwrap());
        assert!(!d.teclado_permitido(&p.token, AGORA));
        assert!(!d.definir_teclado("naoexiste", true).unwrap(), "id desconhecido");
    }

    #[test]
    fn a_permissao_sobrevive_ao_reinicio_e_arquivos_antigos_valem_como_negado() {
        let pasta = Pasta::nova("perm-teclado");
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        let p = d.parear_com("iPhone", 1000, true).unwrap();
        assert!(Dispositivos::com_arquivo(pasta.arquivo()).teclado_permitido(&p.token, AGORA));

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
        assert!(d.autorizado(token, AGORA), "continua pareado");
        assert!(!d.teclado_permitido(token, AGORA), "mas sem teclado até o dono do PC liberar");
    }

    #[test]
    fn mudar_a_permissao_por_outro_processo_vale_depois_de_recarregar() {
        let pasta = Pasta::nova("perm-recarrega");
        let servidor = Dispositivos::com_arquivo(pasta.arquivo());
        let p = servidor.parear_com("iPhone", 1000, true).unwrap();

        Dispositivos::com_arquivo(pasta.arquivo()).definir_teclado(&p.id, false).unwrap();

        assert!(servidor.teclado_permitido(&p.token, AGORA), "ainda não leu o arquivo");
        servidor.recarregar();
        assert!(!servidor.teclado_permitido(&p.token, AGORA));
    }

    #[test]
    fn revogar_o_aparelho_tira_tambem_a_permissao() {
        let d = Dispositivos::em_memoria();
        let p = d.parear_com("iPhone", 1000, true).unwrap();
        d.revogar(&p.id).unwrap();
        assert!(!d.teclado_permitido(&p.token, AGORA));
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
        assert!(!d.teclado_permitido(&p.token, AGORA));
    }

    // ---------- consulta única (autorizado + permissões) ----------

    #[test]
    fn consultar_devolve_as_permissoes_do_aparelho_ou_nada_se_nao_for_pareado() {
        let d = Dispositivos::em_memoria();
        let sem = d.parear("iPad", 1000).unwrap();
        let com = d.parear_com("iPhone", 1001, true).unwrap();
        assert_eq!(d.consultar(&sem.token, AGORA), Some(Permissoes { teclado: false }));
        assert_eq!(d.consultar(&com.token, AGORA), Some(Permissoes { teclado: true }));
        assert_eq!(d.consultar("errado", AGORA), None);
        assert_eq!(d.consultar("", AGORA), None);
    }

    #[test]
    fn consultar_acompanha_a_revogacao_e_a_mudanca_de_permissao() {
        let d = Dispositivos::em_memoria();
        let p = d.parear_com("iPhone", 1000, true).unwrap();
        d.definir_teclado(&p.id, false).unwrap();
        assert_eq!(d.consultar(&p.token, AGORA), Some(Permissoes { teclado: false }));
        d.revogar(&p.id).unwrap();
        assert_eq!(d.consultar(&p.token, AGORA), None);
    }

    #[test]
    fn identificar_devolve_id_e_nome_so_para_tokens_pareados() {
        let d = Dispositivos::em_memoria();
        let p = d.parear_com("iPhone do Wilian", 1000, true).unwrap();
        let r = d.identificar(&p.token, AGORA).unwrap();
        assert_eq!((r.id.as_str(), r.nome.as_str(), r.teclado), (p.id.as_str(), "iPhone do Wilian", true));
        assert!(d.identificar("errado", AGORA).is_none());
    }

    // ---------- validade e visto por último ----------

    #[test]
    fn o_token_vale_ate_a_validade_e_depois_nao_vale_mais() {
        let d = Dispositivos::em_memoria();
        let p = d.parear("iPhone", 1000).unwrap();
        let fim = 1000 + VALIDADE_DO_TOKEN;
        assert!(d.autorizado(&p.token, fim - 1));
        assert!(!d.autorizado(&p.token, fim));
        assert!(d.consultar(&p.token, fim).is_none());
        assert!(d.identificar(&p.token, fim).is_none());
        assert!(!d.teclado_permitido(&p.token, fim));
    }

    #[test]
    fn identificar_registra_quando_o_aparelho_foi_visto_por_ultimo() {
        let d = Dispositivos::em_memoria();
        let p = d.parear("iPhone", 1000).unwrap();
        assert_eq!(d.listar()[0].ultimo_visto, None);
        assert_eq!(d.identificar(&p.token, 5000).unwrap().ultimo_visto, Some(5000));
        assert_eq!(d.listar()[0].ultimo_visto, Some(5000));
        // uma consulta mais antiga não faz o relógio andar para trás
        assert_eq!(d.identificar(&p.token, 4000).unwrap().ultimo_visto, Some(5000));
    }

    #[test]
    fn o_ultimo_visto_sobrevive_ao_reinicio() {
        let pasta = Pasta::nova("visto");
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        let p = d.parear("iPhone", 1000).unwrap();
        d.identificar(&p.token, 5000).unwrap();
        let depois = Dispositivos::com_arquivo(pasta.arquivo());
        assert_eq!(depois.listar()[0].ultimo_visto, Some(5000));
    }

    #[test]
    fn arquivo_das_versoes_anteriores_ganha_validade_a_partir_da_criacao() {
        let pasta = Pasta::nova("antigo");
        let d = Dispositivos::com_arquivo(pasta.arquivo());
        let p = d.parear("iPhone", 1000).unwrap();
        // formato antigo, sem `expira` nem `visto`
        let antigo = format!(
            r#"[{{"id":"{}","nome":"iPhone","hash":"{}","criado":1000}}]"#,
            p.id,
            hash_do_token(&p.token)
        );
        fs::write(pasta.arquivo(), antigo).unwrap();
        let carregado = Dispositivos::com_arquivo(pasta.arquivo());
        assert_eq!(carregado.listar()[0].expira, 1000 + VALIDADE_DO_TOKEN);
        assert!(carregado.autorizado(&p.token, 1000 + VALIDADE_DO_TOKEN - 1));
        assert!(!carregado.autorizado(&p.token, 1000 + VALIDADE_DO_TOKEN));
    }
}
