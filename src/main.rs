use std::{
    io::{self, Write},
    net::IpAddr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::sync::{mpsc, watch};
use vpad::{
    acesso::Acesso,
    armazem::{ArmazemMapa, caminho_da_config},
    cli::{linha_do_codigo, tabela_de_dispositivos},
    configuracao::{self, Configuracao},
    convite::{qr_em_texto, url_de_configuracao, url_do_controle},
    dispositivo::{Gamepad, Teclado},
    dispositivos::{Dispositivos, caminho_dos_dispositivos},
    https,
    pareamento::Pareamento,
    perfil::{Acao, Alvo, Tipo, indice_do_perfil, opcoes_dos_argumentos},
    servico::{self, Dispositivo, Fabrica},
    tls::{carregar_ou_criar, config_do_servidor, nome_do_computador, pasta_do_tls},
};

/// O celular manda um ping por segundo; sem sinal por tanto tempo, a conexão é encerrada.
const INATIVIDADE: Duration = Duration::from_secs(6);
/// De quanto em quanto tempo o servidor relê os aparelhos pareados (o `--revogar` roda em outro processo).
const RECARGA_DOS_DISPOSITIVOS: Duration = Duration::from_secs(2);

/// Endereços IPv4 desta máquina na rede (o certificado precisa cobri-los para o celular confiar).
fn ips_da_rede() -> Vec<IpAddr> {
    local_ip_address::list_afinet_netifas()
        .map(|lista| {
            lista
                .into_iter()
                .filter_map(|(_, ip)| match ip {
                    IpAddr::V4(v4) if !v4.is_loopback() && !v4.is_link_local() => Some(ip),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn agora() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let opcoes = opcoes_dos_argumentos(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let perfil = opcoes.perfil;
    let porta = opcoes.porta;
    let porta_https = opcoes.porta_https;

    let xdg = std::env::var("XDG_CONFIG_HOME").ok();
    let home = std::env::var("HOME").ok();

    let dispositivos = match caminho_dos_dispositivos(xdg.as_deref(), home.as_deref()) {
        Some(arquivo) => Dispositivos::com_arquivo(arquivo),
        None => {
            eprintln!("aviso: sem HOME, os aparelhos pareados não serão salvos");
            Dispositivos::em_memoria()
        }
    };

    match opcoes.acao {
        Acao::ListarDispositivos => {
            println!("{}", tabela_de_dispositivos(&dispositivos.listar()));
            return Ok(());
        }
        Acao::Revogar(Alvo::Todos) => {
            println!("{} dispositivo(s) revogado(s)", dispositivos.revogar_todos()?);
            return Ok(());
        }
        Acao::Revogar(Alvo::Id(id)) => {
            if dispositivos.revogar(&id)? {
                println!("dispositivo {id} revogado");
                return Ok(());
            }
            eprintln!("nenhum dispositivo com o id {id} (veja os ids com: vpad --dispositivos)");
            std::process::exit(1);
        }
        Acao::Servir => {}
    }

    let acesso = Acesso::novo(dispositivos, Pareamento::novo()?, INATIVIDADE);

    let armazem = Arc::new(match caminho_da_config(xdg.as_deref(), home.as_deref()) {
        Some(arquivo) => {
            println!("teclas salvas em {}", arquivo.display());
            ArmazemMapa::com_arquivo(arquivo)
        }
        None => ArmazemMapa::em_memoria(),
    });

    let fabrica: Fabrica = {
        let armazem = armazem.clone();
        Box::new(move |p| match p.tipo {
            Tipo::Gamepad => Gamepad::novo(p).map(|g| Box::new(g) as Box<dyn Dispositivo>),
            Tipo::Teclado => {
                Teclado::novo(p, (*armazem.atual()).clone()).map(|t| Box::new(t) as Box<dyn Dispositivo>)
            }
        })
    };
    let dispositivo = fabrica(perfil).map_err(|e| {
        eprintln!("não consegui criar o controle virtual: {e}");
        eprintln!("verifique se o módulo uinput está carregado (sudo modprobe uinput)");
        eprintln!("e se seu usuário tem permissão de escrita em /dev/uinput");
        e
    })?;

    let (tx, rx) = mpsc::unbounded_channel();
    let (perfil_tx, perfil_rx) = watch::channel(indice_do_perfil(perfil.id).unwrap_or(0));
    tokio::spawn(servico::processar(rx, perfil_tx, armazem.assinar(), fabrica, dispositivo));

    // Aparelhos revogados por `vpad --revogar` (outro processo) deixam de valer aqui também.
    let para_recarregar = acesso.clone();
    tokio::spawn(async move {
        let mut intervalo = tokio::time::interval(RECARGA_DOS_DISPOSITIVOS);
        loop {
            intervalo.tick().await;
            para_recarregar.dispositivos().recarregar();
        }
    });

    // HTTPS com uma autoridade própria: o celular a instala uma vez (pela página de configuração).
    let Some(pasta_tls) = pasta_do_tls(xdg.as_deref(), home.as_deref()) else {
        eprintln!("sem HOME não há onde guardar o certificado; defina HOME ou XDG_CONFIG_HOME");
        std::process::exit(1);
    };
    let ips = ips_da_rede();
    let material = carregar_ou_criar(&pasta_tls, &ips, agora())?;
    let tls_config = config_do_servidor(&material)?;

    // Porta HTTP: só entrega o certificado e as instruções. O controle fica no HTTPS.
    let listener_http = tokio::net::TcpListener::bind(("0.0.0.0", porta)).await?;
    let configuracao = Arc::new(Configuracao {
        ca_der: material.ca_der.clone(),
        ca_pem: material.ca_pem.clone(),
        nome_do_pc: nome_do_computador(),
        porta_https,
    });
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener_http, configuracao::rotas(configuracao)).await {
            eprintln!("erro na página de configuração: {e}");
        }
    });
    let listener_https = tokio::net::TcpListener::bind(("0.0.0.0", porta_https)).await?;

    let ip = local_ip_address::local_ip().map(|i| i.to_string()).unwrap_or_else(|_| "IP_DO_PC".into());
    let url = url_do_controle(&ip, porta_https);
    println!("{}", qr_em_texto(&url));
    println!("perfil: {} ({:04x}:{:04x})", perfil.nome, perfil.fabricante, perfil.produto);
    println!("controle (HTTPS): {url}");
    println!("primeira vez neste celular? instale o certificado: {}", url_de_configuracao(&ip, porta));
    if material.ca_nova {
        println!("AUTORIDADE DE CERTIFICAÇÃO NOVA: instale o perfil de novo em cada celular que já tinha o antigo");
    } else if material.folha_renovada {
        println!("certificado do servidor renovado (IP novo ou perto de vencer); os celulares não precisam fazer nada");
    }
    println!("certificado válido para: {}", ips.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "));
    println!("aparelhos pareados: {} (veja com: vpad --dispositivos)", acesso.dispositivos().listar().len());
    println!("para parear um aparelho novo, digite no celular o código abaixo:\n");

    // O código de pareamento muda a cada 30 s; esta linha é reescrita a cada segundo.
    let para_mostrar = acesso.clone();
    tokio::spawn(async move {
        let mut intervalo = tokio::time::interval(Duration::from_secs(1));
        loop {
            intervalo.tick().await;
            let agora = agora();
            let p = para_mostrar.pareamento();
            print!("{}", linha_do_codigo(&p.codigo_atual(agora), p.segundos_restantes(agora)));
            let _ = io::stdout().flush();
        }
    });

    https::servir(listener_https, tls_config, vpad::app_com(tx, perfil_rx, armazem, acesso)).await
}
