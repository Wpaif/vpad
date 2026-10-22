use std::{
    io::{self, Write},
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::sync::{mpsc, watch};
use vpad::{
    acesso::Acesso,
    avisos,
    armazem::{ArmazemMapa, caminho_da_config},
    layout::{ArmazemLayout, caminho_do_layout},
    cli::tabela_de_dispositivos,
    configuracao::{Configuracao, ServidorDeConfiguracao},
    console::{Console, DURACAO_DA_CONFIGURACAO, DURACAO_DO_PAREAMENTO, Saida},
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

/// Mostra o que o console mandou: linhas permanentes ou a linha do código que se reescreve.
fn imprimir(saidas: Vec<Saida>) {
    for saida in saidas {
        match saida {
            Saida::Linha(texto) => println!("\r\x1b[2K{texto}"),
            Saida::Atualiza(texto) => print!("{texto}"),
        }
    }
    let _ = io::stdout().flush();
}

fn agora() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // o idioma do terminal vem antes de tudo: até os erros dos argumentos saem nele
    vpad::mensagens::definir_idioma_do_terminal(vpad::mensagens::detectar(|nome| std::env::var(nome).ok()));
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
            eprintln!("{}", avisos::sem_home_para_aparelhos());
            Dispositivos::em_memoria()
        }
    };

    match opcoes.acao {
        Acao::ListarDispositivos => {
            println!("{}", tabela_de_dispositivos(&dispositivos.listar()));
            return Ok(());
        }
        Acao::Revogar(Alvo::Todos) => {
            println!("{}", avisos::dispositivos_revogados(dispositivos.revogar_todos()?));
            return Ok(());
        }
        Acao::Revogar(Alvo::Id(id)) => {
            if dispositivos.revogar(&id)? {
                println!("{}", avisos::dispositivo_revogado(&id));
                return Ok(());
            }
            eprintln!("{}", avisos::nenhum_dispositivo_com_o_id(&id));
            std::process::exit(1);
        }
        Acao::Teclado { id, permitir } => {
            if dispositivos.definir_teclado(&id, permitir)? {
                println!("{}", avisos::modo_teclado_do_dispositivo(permitir, &id));
                return Ok(());
            }
            eprintln!("{}", avisos::nenhum_dispositivo_com_o_id(&id));
            std::process::exit(1);
        }
        Acao::Servir => {}
    }

    // Alertas de segurança vão para o terminal (limpando antes a linha do código que gira).
    let acesso = Acesso::com_alertas(
        dispositivos,
        Pareamento::novo()?,
        INATIVIDADE,
        Arc::new(|alerta| {
            println!("\r\x1b[2K{alerta}");
        }),
    );

    let armazem = Arc::new(match caminho_da_config(xdg.as_deref(), home.as_deref()) {
        Some(arquivo) => {
            println!("{}", avisos::teclas_salvas_em(&arquivo.display().to_string()));
            ArmazemMapa::com_arquivo(arquivo)
        }
        None => ArmazemMapa::em_memoria(),
    });

    let layout = Arc::new(match caminho_do_layout(xdg.as_deref(), home.as_deref()) {
        Some(arquivo) => {
            println!("{}", avisos::layout_salvo_em(&arquivo.display().to_string()));
            ArmazemLayout::com_arquivo(arquivo)
        }
        None => ArmazemLayout::em_memoria(),
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
        for linha in avisos::nao_criou_o_controle_virtual(&e.to_string()) {
            eprintln!("{linha}");
        }
        e
    })?;

    let (tx, rx) = mpsc::channel(1024);
    let (perfil_tx, perfil_rx) = watch::channel(indice_do_perfil(perfil.id).unwrap_or(0));
    tokio::spawn(servico::processar(rx, perfil_tx, armazem.assinar(), fabrica, dispositivo));

    // Aparelhos revogados por `vpad --revogar` (outro processo) deixam de valer aqui também:
    // o inotify reage na hora e a releitura periódica cobre o caso de ele não estar disponível.
    let para_recarregar = acesso.clone();
    tokio::spawn(async move {
        let mut intervalo = tokio::time::interval(RECARGA_DOS_DISPOSITIVOS);
        loop {
            intervalo.tick().await;
            para_recarregar.dispositivos().recarregar();
        }
    });
    match vpad::vigiar_revogacoes(acesso.clone()) {
        Ok(_) => {}
        Err(e) => eprintln!("{}", avisos::sem_inotify(&e.to_string(), RECARGA_DOS_DISPOSITIVOS.as_secs())),
    }

    // HTTPS com uma autoridade própria: o celular a instala uma vez (pela página de configuração).
    let Some(pasta_tls) = pasta_do_tls(xdg.as_deref(), home.as_deref()) else {
        eprintln!("{}", avisos::sem_home_para_o_certificado());
        std::process::exit(1);
    };
    let ips = ips_da_rede();
    let material = carregar_ou_criar(&pasta_tls, &ips, agora())?;
    let tls_config = config_do_servidor(&material)?;

    // A página que entrega o certificado (HTTP simples) só abre quando o dono do PC pede.
    let configuracao = Arc::new(Configuracao {
        ca_der: material.ca_der.clone(),
        ca_pem: material.ca_pem.clone(),
        nome_do_pc: nome_do_computador(),
        porta_https,
    });
    let servidor_de_configuracao = ServidorDeConfiguracao::novo(SocketAddr::from(([0, 0, 0, 0], porta)), configuracao);
    let listener_https = tokio::net::TcpListener::bind(("0.0.0.0", porta_https)).await?;

    let ip = local_ip_address::local_ip().map(|i| i.to_string()).unwrap_or_else(|_| "IP_DO_PC".into());
    let url = url_do_controle(&ip, porta_https);
    let url_config = url_de_configuracao(&ip, porta);
    println!("{}", qr_em_texto(&url));
    println!("{}", avisos::linha_do_perfil(perfil.nome, perfil.fabricante, perfil.produto));
    println!("{}", avisos::controle_https(&url));
    if material.ca_antiga_sem_restricoes {
        for linha in avisos::autoridade_antiga_substituida() {
            println!("{linha}");
        }
    } else if material.ca_nova {
        println!("{}", avisos::autoridade_nova());
    } else if material.folha_renovada {
        println!("{}", avisos::certificado_renovado());
    }
    let impressao = material.impressao_digital();
    let (primeira, segunda) = impressao.split_at(impressao.char_indices().filter(|(_, c)| *c == ':').nth(15).map_or(0, |(i, _)| i + 1));
    println!("{}", avisos::impressao_digital(primeira, segunda));
    for ip in &material.ips_ignorados {
        eprintln!("{}", avisos::ip_fora_do_certificado(&ip.to_string()));
    }
    let cobertos: Vec<String> = ips.iter().filter(|i| !material.ips_ignorados.contains(i)).map(ToString::to_string).collect();
    println!("{}", avisos::certificado_valido_para(&cobertos.join(", ")));
    println!("{}", avisos::aparelhos_pareados(acesso.dispositivos().listar().len()));

    // Primeira instalação do certificado: abre a página de configuração por um tempo.
    if material.ca_nova {
        match servidor_de_configuracao.abrir(DURACAO_DA_CONFIGURACAO) {
            Ok(_) => println!("{}", avisos::pagina_de_configuracao_aberta(DURACAO_DA_CONFIGURACAO.as_secs(), &url_config)),
            Err(e) => eprintln!("{}", avisos::nao_abriu_a_pagina_de_configuracao(&e.to_string())),
        }
    }
    if opcoes.parear_ao_iniciar {
        acesso.pareamento().abrir(agora(), DURACAO_DO_PAREAMENTO);
    }
    println!();

    // O terminal é o painel de segurança: Enter abre o pareamento, c abre a página do
    // certificado, s/t/outra tecla respondem aos pedidos de aprovação.
    let console = Arc::new(Console::novo(acesso.clone(), Arc::new(servidor_de_configuracao.clone()), url_config));
    let para_o_tique = console.clone();
    tokio::spawn(async move {
        let mut intervalo = tokio::time::interval(Duration::from_secs(1));
        loop {
            intervalo.tick().await;
            imprimir(para_o_tique.tique(agora()));
        }
    });
    let para_a_entrada = console.clone();
    tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let mut linhas = BufReader::new(tokio::io::stdin()).lines();
        while let Ok(Some(linha)) = linhas.next_line().await {
            imprimir(para_a_entrada.linha(&linha, agora()));
        }
        // entrada fechada (rodando sem terminal): só dá para parear com --parear
        println!("{}", avisos::sem_entrada_de_terminal());
    });

    https::servir(listener_https, tls_config, vpad::app_completo(tx, perfil_rx, armazem, layout, acesso)).await
}
