//! HTTPS na rede local. Certificado comum só vale com domínio público, então o vpad cria uma
//! autoridade própria (que o celular instala uma vez) e, com ela, o certificado do PC. A
//! autoridade dura anos e não muda; o certificado do PC é refeito sozinho quando o IP da
//! máquina muda ou quando está perto de vencer, sem o celular precisar reinstalar nada.

use std::{
    fs,
    io::{self, Write},
    net::IpAddr,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose, SanType,
};
use rustls::{
    ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

const DIA: u64 = 86_400;
/// Com o dia de folga no começo, o certificado dura 397 dias: abaixo do limite de 825 que o
/// iOS impõe a certificados de servidor (e do de 398 dos navegadores para autoridades públicas).
const VALIDADE_DO_CERTIFICADO: u64 = 396 * DIA;
const VALIDADE_DA_AUTORIDADE: u64 = 10 * 365 * DIA;
/// Falta menos que isso para vencer: renova.
const RENOVAR_ANTES_DE: u64 = 30 * DIA;

pub struct Material {
    /// certificado da autoridade, o que o celular instala
    pub ca_pem: String,
    pub ca_der: Vec<u8>,
    /// certificado do servidor e a chave dele
    pub folha_pem: String,
    pub chave_pem: String,
    /// a autoridade foi criada agora: celulares que instalaram a antiga precisam instalar esta
    pub ca_nova: bool,
    pub folha_renovada: bool,
}

#[derive(Serialize, Deserialize)]
struct Anotacao {
    ips: Vec<String>,
    /// fim da validade do certificado do servidor (segundos desde 1970)
    expira: u64,
}

fn erro(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}

fn instante(segundos: u64) -> io::Result<OffsetDateTime> {
    OffsetDateTime::from_unix_timestamp(segundos as i64).map_err(erro)
}

/// Grava com permissão restrita (chaves) usando arquivo temporário e renomeação.
fn escrever(arquivo: &Path, conteudo: &str, privado: bool) -> io::Result<()> {
    let temporario = arquivo.with_extension("tmp");
    let mut f = fs::File::create(&temporario)?;
    if privado {
        f.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    f.write_all(conteudo.as_bytes())?;
    drop(f);
    fs::rename(&temporario, arquivo)
}

pub fn nome_do_computador() -> String {
    fs::read_to_string("/etc/hostname")
        .ok()
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "pc".to_string())
}

/// `$XDG_CONFIG_HOME/vpad/tls`, ou `~/.config/vpad/tls`.
pub fn pasta_do_tls(xdg: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    let base = match xdg.filter(|x| !x.is_empty()) {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from(home.filter(|h| !h.is_empty())?).join(".config"),
    };
    Some(base.join("vpad").join("tls"))
}

struct Autoridade {
    pem: String,
    der: Vec<u8>,
    emissor: Issuer<'static, KeyPair>,
    nova: bool,
}

fn criar_autoridade(pasta: &Path, agora: u64) -> io::Result<Autoridade> {
    let mut params = CertificateParams::new(Vec::<String>::new()).map_err(erro)?;
    params.distinguished_name.push(DnType::CommonName, format!("vpad local CA ({})", nome_do_computador()));
    params.distinguished_name.push(DnType::OrganizationName, "vpad");
    params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0)); // só assina certificados finais
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    params.not_before = instante(agora - DIA)?;
    params.not_after = instante(agora + VALIDADE_DA_AUTORIDADE)?;

    let chave = KeyPair::generate().map_err(erro)?;
    let cert = params.self_signed(&chave).map_err(erro)?;
    let pem = cert.pem();
    escrever(&pasta.join("ca.pem"), &pem, false)?;
    escrever(&pasta.join("ca-chave.pem"), &chave.serialize_pem(), true)?;
    Ok(Autoridade {
        der: cert.der().to_vec(),
        emissor: Issuer::new(params, chave),
        pem,
        nova: true,
    })
}

fn carregar_autoridade(pasta: &Path) -> Option<Autoridade> {
    let pem = fs::read_to_string(pasta.join("ca.pem")).ok()?;
    let chave = KeyPair::from_pem(&fs::read_to_string(pasta.join("ca-chave.pem")).ok()?).ok()?;
    let der = CertificateDer::from_pem_slice(pem.as_bytes()).ok()?.to_vec();
    let emissor = Issuer::from_ca_cert_pem(&pem, chave).ok()?;
    Some(Autoridade { pem, der, emissor, nova: false })
}

/// IPs que o certificado precisa cobrir: os pedidos mais o de loopback.
fn ips_necessarios(ips: &[IpAddr]) -> Vec<String> {
    let mut lista: Vec<String> = ips.iter().map(|i| i.to_string()).collect();
    lista.push("127.0.0.1".to_string());
    lista.sort();
    lista.dedup();
    lista
}

fn criar_folha(pasta: &Path, autoridade: &Autoridade, ips: &[String], agora: u64) -> io::Result<()> {
    let mut params = CertificateParams::new(Vec::<String>::new()).map_err(erro)?;
    params.distinguished_name.push(DnType::CommonName, "vpad");
    let mut nomes = vec![SanType::DnsName("localhost".try_into().map_err(erro)?)];
    for ip in ips {
        nomes.push(SanType::IpAddress(ip.parse().map_err(erro)?));
    }
    params.subject_alt_names = nomes;
    params.is_ca = IsCa::NoCa;
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    params.use_authority_key_identifier_extension = true;
    // começa um dia antes: o relógio do celular pode estar um pouco atrasado
    params.not_before = instante(agora - DIA)?;
    params.not_after = instante(agora + VALIDADE_DO_CERTIFICADO)?;

    let chave = KeyPair::generate().map_err(erro)?;
    let cert = params.signed_by(&chave, &autoridade.emissor).map_err(erro)?;
    escrever(&pasta.join("servidor.pem"), &cert.pem(), false)?;
    escrever(&pasta.join("servidor-chave.pem"), &chave.serialize_pem(), true)?;
    let anotacao = Anotacao { ips: ips.to_vec(), expira: agora + VALIDADE_DO_CERTIFICADO };
    escrever(&pasta.join("servidor.json"), &serde_json::to_string_pretty(&anotacao).map_err(erro)?, false)
}

/// O certificado guardado ainda serve? (existe, é legível, cobre os IPs e não está para vencer)
fn folha_ainda_serve(pasta: &Path, ips: &[String], agora: u64) -> bool {
    let Ok(texto) = fs::read_to_string(pasta.join("servidor.json")) else { return false };
    let Ok(anotacao) = serde_json::from_str::<Anotacao>(&texto) else { return false };
    let cobre = ips.iter().all(|ip| anotacao.ips.contains(ip));
    let valido = agora + RENOVAR_ANTES_DE < anotacao.expira;
    let legivel = fs::read(pasta.join("servidor.pem"))
        .is_ok_and(|pem| CertificateDer::from_pem_slice(&pem).is_ok())
        && fs::read(pasta.join("servidor-chave.pem"))
            .is_ok_and(|pem| PrivateKeyDer::from_pem_slice(&pem).is_ok());
    cobre && valido && legivel
}

/// Carrega a autoridade e o certificado de `pasta`, criando ou renovando o que faltar.
pub fn carregar_ou_criar(pasta: &Path, ips: &[IpAddr], agora: u64) -> io::Result<Material> {
    fs::create_dir_all(pasta)?;
    let necessarios = ips_necessarios(ips);

    let autoridade = match carregar_autoridade(pasta) {
        Some(a) => a,
        None => criar_autoridade(pasta, agora)?,
    };
    // autoridade nova invalida o certificado antigo: ele foi assinado pela anterior
    let renovar = autoridade.nova || !folha_ainda_serve(pasta, &necessarios, agora);
    if renovar {
        let ips_do_certificado = if autoridade.nova { necessarios } else { uniao_com_o_antigo(pasta, &necessarios) };
        criar_folha(pasta, &autoridade, &ips_do_certificado, agora)?;
    }

    Ok(Material {
        ca_pem: autoridade.pem,
        ca_der: autoridade.der,
        folha_pem: fs::read_to_string(pasta.join("servidor.pem"))?,
        chave_pem: fs::read_to_string(pasta.join("servidor-chave.pem"))?,
        ca_nova: autoridade.nova,
        folha_renovada: renovar,
    })
}

/// Ao renovar, mantém também os IPs do certificado anterior: trocar de rede e voltar
/// não deve obrigar a renovar de novo.
fn uniao_com_o_antigo(pasta: &Path, necessarios: &[String]) -> Vec<String> {
    let antigos = fs::read_to_string(pasta.join("servidor.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Anotacao>(&t).ok())
        .map(|a| a.ips)
        .unwrap_or_default();
    let mut todos: Vec<String> = necessarios.iter().cloned().chain(antigos).collect();
    todos.sort();
    todos.dedup();
    todos
}

/// Configuração TLS do servidor, oferecendo HTTP/2 e HTTP/1.1.
pub fn config_do_servidor(material: &Material) -> io::Result<Arc<ServerConfig>> {
    let cadeia = CertificateDer::pem_slice_iter(material.folha_pem.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .map_err(erro)?;
    let chave = PrivateKeyDer::from_pem_slice(material.chave_pem.as_bytes()).map_err(erro)?;
    let mut config = ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(erro)?
        .with_no_client_auth()
        .with_single_cert(cadeia, chave)
        .map_err(erro)?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::{
        fs,
        net::{IpAddr, Ipv4Addr},
        os::unix::fs::PermissionsExt,
        sync::Arc,
    };
    use rustls::{
        ClientConfig, RootCertStore,
        pki_types::{CertificateDer, ServerName, pem::PemObject},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_rustls::{TlsAcceptor, TlsConnector};
    use x509_parser::{
        extensions::GeneralName,
        prelude::{FromDer, X509Certificate},
    };

    const DIA: u64 = 86_400;

    /// A hora de verdade: o rustls confere a validade com o relógio do sistema.
    fn agora() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
    }

    fn ip(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(a, b, c, d))
    }

    struct Pasta(PathBuf);
    impl Pasta {
        fn nova(nome: &str) -> Pasta {
            let p = std::env::temp_dir().join(format!("vpad-tls-{}-{nome}", std::process::id()));
            let _ = fs::remove_dir_all(&p);
            Pasta(p.join("tls")) // não existe ainda: a função precisa criá-la
        }
    }
    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(self.0.parent().unwrap());
        }
    }

    fn folha_der(m: &Material) -> Vec<u8> {
        CertificateDer::pem_slice_iter(m.folha_pem.as_bytes()).next().unwrap().unwrap().to_vec()
    }

    /// Handshake TLS completo em memória: o cliente confia em `confiar` e quer falar com `nome`.
    async fn handshake(m: &Material, confiar: &[Vec<u8>], nome: ServerName<'static>) -> Result<(), String> {
        let (cliente, servidor) = tokio::io::duplex(16 * 1024);
        let acceptor = TlsAcceptor::from(config_do_servidor(m).map_err(|e| e.to_string())?);
        let tarefa = tokio::spawn(async move {
            let mut t = acceptor.accept(servidor).await.map_err(|e| e.to_string())?;
            t.write_all(b"ok").await.map_err(|e| e.to_string())?;
            t.shutdown().await.ok();
            Ok::<(), String>(())
        });

        let mut raizes = RootCertStore::empty();
        for der in confiar {
            raizes.add(CertificateDer::from(der.clone())).unwrap();
        }
        let config = ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(raizes)
            .with_no_client_auth();
        let mut t = TlsConnector::from(Arc::new(config))
            .connect(nome, cliente)
            .await
            .map_err(|e| e.to_string())?;
        let mut buf = [0u8; 2];
        t.read_exact(&mut buf).await.map_err(|e| e.to_string())?;
        tarefa.await.unwrap()
    }

    fn por_ip(a: IpAddr) -> ServerName<'static> {
        ServerName::IpAddress(a.into())
    }

    // ---------- criação ----------

    #[test]
    fn cria_a_pasta_a_autoridade_e_o_certificado_na_primeira_vez() {
        let pasta = Pasta::nova("cria");
        let m = carregar_ou_criar(&pasta.0, &[ip(192, 168, 68, 103)], agora()).unwrap();
        assert!(m.ca_nova && m.folha_renovada);
        for arquivo in ["ca.pem", "ca-chave.pem", "servidor.pem", "servidor-chave.pem", "servidor.json"] {
            assert!(pasta.0.join(arquivo).exists(), "{arquivo}");
        }
    }

    #[test]
    fn as_chaves_ficam_legiveis_so_para_o_dono() {
        let pasta = Pasta::nova("perm");
        carregar_ou_criar(&pasta.0, &[ip(127, 0, 0, 1)], agora()).unwrap();
        for arquivo in ["ca-chave.pem", "servidor-chave.pem"] {
            let modo = fs::metadata(pasta.0.join(arquivo)).unwrap().permissions().mode() & 0o777;
            assert_eq!(modo, 0o600, "{arquivo}");
        }
    }

    // ---------- confiança ----------

    #[tokio::test]
    async fn quem_confia_na_autoridade_conecta_pelo_ip_da_rede_e_por_localhost() {
        let pasta = Pasta::nova("hs-ok");
        let rede = ip(192, 168, 68, 103);
        let m = carregar_ou_criar(&pasta.0, &[rede], agora()).unwrap();
        let ca = vec![m.ca_der.clone()];

        handshake(&m, &ca, por_ip(rede)).await.unwrap();
        handshake(&m, &ca, por_ip(ip(127, 0, 0, 1))).await.unwrap();
        handshake(&m, &ca, ServerName::try_from("localhost").unwrap()).await.unwrap();
    }

    #[tokio::test]
    async fn sem_confiar_na_autoridade_o_certificado_e_recusado() {
        let pasta = Pasta::nova("hs-sem-ca");
        let rede = ip(192, 168, 68, 103);
        let m = carregar_ou_criar(&pasta.0, &[rede], agora()).unwrap();
        assert!(handshake(&m, &[], por_ip(rede)).await.is_err());
    }

    #[tokio::test]
    async fn ip_que_nao_esta_no_certificado_e_recusado() {
        let pasta = Pasta::nova("hs-outro-ip");
        let m = carregar_ou_criar(&pasta.0, &[ip(192, 168, 68, 103)], agora()).unwrap();
        let ca = vec![m.ca_der.clone()];
        assert!(handshake(&m, &ca, por_ip(ip(10, 0, 0, 5))).await.is_err());
    }

    #[test]
    fn o_servidor_oferece_http2_e_http1() {
        let pasta = Pasta::nova("alpn");
        let m = carregar_ou_criar(&pasta.0, &[ip(127, 0, 0, 1)], agora()).unwrap();
        let config = config_do_servidor(&m).unwrap();
        assert_eq!(config.alpn_protocols, vec![b"h2".to_vec(), b"http/1.1".to_vec()]);
    }

    // ---------- requisitos do iOS ----------

    #[test]
    fn a_autoridade_e_o_certificado_cumprem_o_que_o_ios_exige() {
        let pasta = Pasta::nova("ios");
        let rede = ip(192, 168, 68, 103);
        let m = carregar_ou_criar(&pasta.0, &[rede, ip(172, 17, 0, 1)], agora()).unwrap();

        let (_, ca) = X509Certificate::from_der(&m.ca_der).unwrap();
        let bc = ca.basic_constraints().unwrap().unwrap().value;
        assert!(bc.ca && bc.path_len_constraint == Some(0), "a autoridade só assina certificados finais");
        assert!(ca.key_usage().unwrap().unwrap().value.key_cert_sign());
        assert!(ca.subject().to_string().contains("vpad"));
        let anos = (ca.validity().not_after.timestamp() - ca.validity().not_before.timestamp()) / (365 * DIA as i64);
        assert!(anos >= 5, "a autoridade precisa durar para o celular não reinstalar toda hora: {anos} anos");

        let der = folha_der(&m);
        let (_, folha) = X509Certificate::from_der(&der).unwrap();
        assert!(folha.basic_constraints().unwrap().is_none_or(|b| !b.value.ca), "o certificado final não é autoridade");
        assert!(folha.extended_key_usage().unwrap().unwrap().value.server_auth);
        assert!(folha.key_usage().unwrap().unwrap().value.digital_signature());

        let san = folha.subject_alternative_name().unwrap().unwrap().value;
        let tem_ip = |a: IpAddr| {
            san.general_names.iter().any(|n| match (n, a) {
                (GeneralName::IPAddress(b), IpAddr::V4(v4)) => *b == v4.octets(),
                _ => false,
            })
        };
        assert!(tem_ip(rede) && tem_ip(ip(172, 17, 0, 1)) && tem_ip(ip(127, 0, 0, 1)));
        assert!(san.general_names.iter().any(|n| matches!(n, GeneralName::DNSName("localhost"))));

        let dias = (folha.validity().not_after.timestamp() - folha.validity().not_before.timestamp()) / DIA as i64;
        assert!(dias <= 397, "o iOS recusa certificado de servidor com validade acima de 825 dias; usamos {dias}");
        assert!(folha.validity().not_before.timestamp() <= agora() as i64, "já valendo, mesmo com relógio um pouco atrasado");
        assert_eq!(folha.signature_algorithm.algorithm.to_id_string(), "1.2.840.10045.4.3.2", "ECDSA com SHA-256");
        assert!(folha.get_extension_unique(&x509_parser::oid_registry::OID_X509_EXT_AUTHORITY_KEY_IDENTIFIER).unwrap().is_some());
    }

    // ---------- reaproveitar e renovar ----------

    #[test]
    fn com_os_mesmos_ips_reaproveita_tudo() {
        let pasta = Pasta::nova("reusa");
        let ips = [ip(192, 168, 68, 103)];
        let a = carregar_ou_criar(&pasta.0, &ips, agora()).unwrap();
        let b = carregar_ou_criar(&pasta.0, &ips, agora() + DIA).unwrap();
        assert!(!b.ca_nova && !b.folha_renovada);
        assert_eq!((a.ca_pem.as_str(), a.folha_pem.as_str()), (b.ca_pem.as_str(), b.folha_pem.as_str()));
    }

    #[tokio::test]
    async fn ip_novo_renova_so_o_certificado_e_o_celular_continua_confiando() {
        let pasta = Pasta::nova("ip-novo");
        let antigo = ip(192, 168, 68, 103);
        let novo = ip(192, 168, 1, 50);
        let a = carregar_ou_criar(&pasta.0, &[antigo], agora()).unwrap();
        let b = carregar_ou_criar(&pasta.0, &[antigo, novo], agora() + DIA).unwrap();

        assert!(!b.ca_nova && b.folha_renovada);
        assert_eq!(a.ca_pem, b.ca_pem, "a autoridade (instalada no celular) não pode mudar");
        assert_ne!(a.folha_pem, b.folha_pem);
        // quem instalou a autoridade antes já confia no certificado novo
        handshake(&b, &[a.ca_der.clone()], por_ip(novo)).await.unwrap();
        handshake(&b, &[a.ca_der.clone()], por_ip(antigo)).await.unwrap();
    }

    #[test]
    fn ter_menos_ips_do_que_o_certificado_cobre_nao_renova() {
        let pasta = Pasta::nova("menos-ips");
        carregar_ou_criar(&pasta.0, &[ip(192, 168, 68, 103), ip(172, 17, 0, 1)], agora()).unwrap();
        let b = carregar_ou_criar(&pasta.0, &[ip(192, 168, 68, 103)], agora() + DIA).unwrap();
        assert!(!b.folha_renovada);
    }

    #[test]
    fn perto_do_vencimento_o_certificado_e_renovado_sem_mexer_na_autoridade() {
        let pasta = Pasta::nova("vence");
        let ips = [ip(192, 168, 68, 103)];
        let a = carregar_ou_criar(&pasta.0, &ips, agora()).unwrap();
        let cedo = carregar_ou_criar(&pasta.0, &ips, agora() + 300 * DIA).unwrap();
        assert!(!cedo.folha_renovada, "ainda falta bastante");
        let tarde = carregar_ou_criar(&pasta.0, &ips, agora() + 380 * DIA).unwrap();
        assert!(tarde.folha_renovada && !tarde.ca_nova);
        assert_eq!(a.ca_pem, tarde.ca_pem);
    }

    #[test]
    fn certificado_corrompido_e_refeito_mas_a_autoridade_fica() {
        let pasta = Pasta::nova("folha-ruim");
        let ips = [ip(192, 168, 68, 103)];
        let a = carregar_ou_criar(&pasta.0, &ips, agora()).unwrap();
        fs::write(pasta.0.join("servidor.pem"), "lixo").unwrap();
        let b = carregar_ou_criar(&pasta.0, &ips, agora()).unwrap();
        assert!(!b.ca_nova && b.folha_renovada);
        assert_eq!(a.ca_pem, b.ca_pem);
    }

    #[tokio::test]
    async fn autoridade_perdida_gera_outra_e_avisa_porque_o_celular_precisa_reinstalar() {
        let pasta = Pasta::nova("ca-perdida");
        let rede = ip(192, 168, 68, 103);
        let a = carregar_ou_criar(&pasta.0, &[rede], agora()).unwrap();
        fs::remove_file(pasta.0.join("ca-chave.pem")).unwrap();
        let b = carregar_ou_criar(&pasta.0, &[rede], agora()).unwrap();

        assert!(b.ca_nova && b.folha_renovada);
        assert_ne!(a.ca_pem, b.ca_pem);
        assert!(handshake(&b, &[a.ca_der.clone()], por_ip(rede)).await.is_err(), "a autoridade antiga não vale mais");
        handshake(&b, &[b.ca_der.clone()], por_ip(rede)).await.unwrap();
    }

    #[test]
    fn a_pasta_do_tls_fica_ao_lado_da_configuracao() {
        assert_eq!(pasta_do_tls(Some("/x"), Some("/h")), Some(PathBuf::from("/x/vpad/tls")));
        assert_eq!(pasta_do_tls(None, Some("/h")), Some(PathBuf::from("/h/.config/vpad/tls")));
        assert_eq!(pasta_do_tls(Some(""), Some("/h")), Some(PathBuf::from("/h/.config/vpad/tls")));
        assert_eq!(pasta_do_tls(None, None), None);
    }

    #[test]
    fn nome_do_computador_nunca_e_vazio() {
        assert!(!nome_do_computador().is_empty());
    }

    #[test]
    fn sem_nenhum_ip_ainda_vale_para_localhost() {
        let pasta = Pasta::nova("sem-ip");
        assert!(carregar_ou_criar(&pasta.0, &[], agora()).is_ok());
    }
}
