use std::{io, sync::Arc, time::Duration};

use axum::serve::ListenerExt;
use tokio::sync::{mpsc, watch};
use vpad::{
    acesso::{Acesso, caminho_do_token, gerar_token, token_do_arquivo},
    armazem::{ArmazemMapa, caminho_da_config},
    convite::{qr_em_texto, url_de_acesso},
    dispositivo::{Gamepad, Teclado},
    perfil::{Tipo, indice_do_perfil, opcoes_dos_argumentos},
    servico::{self, Dispositivo, Fabrica},
};

const PORTA: u16 = 8080;
/// O celular manda um ping por segundo; sem sinal por tanto tempo, a vaga de dono abre.
const INATIVIDADE: Duration = Duration::from_secs(6);

#[tokio::main]
async fn main() -> io::Result<()> {
    let opcoes = opcoes_dos_argumentos(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let perfil = opcoes.perfil;

    let xdg = std::env::var("XDG_CONFIG_HOME").ok();
    let home = std::env::var("HOME").ok();

    // O token fica salvo para o atalho do celular continuar valendo a cada reinício.
    let token = match caminho_do_token(xdg.as_deref(), home.as_deref()) {
        Some(arquivo) => token_do_arquivo(&arquivo, opcoes.novo_token)?,
        None => gerar_token()?,
    };
    let acesso = Acesso::novo(token.clone(), INATIVIDADE);

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

    // Sem Nagle: cada toque vira um pacote imediato.
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", PORTA))
        .await?
        .tap_io(|tcp| {
            let _ = tcp.set_nodelay(true);
        });

    let ip = local_ip_address::local_ip().map(|i| i.to_string()).unwrap_or_else(|_| "IP_DO_PC".into());
    let url = url_de_acesso(&ip, PORTA, &token);
    if opcoes.novo_token {
        println!("novo token gerado: os aparelhos que já tinham o antigo precisam escanear de novo");
    }
    println!("{}", qr_em_texto(&url));
    println!("perfil: {} ({:04x}:{:04x})", perfil.nome, perfil.fabricante, perfil.produto);
    println!("escaneie o QR code ou abra no celular:\n  {url}");
    axum::serve(listener, vpad::app_com(tx, perfil_rx, armazem, acesso)).await
}
