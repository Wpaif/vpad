use std::{io, sync::Arc};

use axum::serve::ListenerExt;
use tokio::sync::{mpsc, watch};
use vpad::{
    armazem::{ArmazemMapa, caminho_da_config},
    dispositivo::{Gamepad, Teclado},
    perfil::{Tipo, indice_do_perfil, perfil_dos_argumentos},
    servico::{self, Dispositivo, Fabrica},
};

const PORTA: u16 = 8080;

#[tokio::main]
async fn main() -> io::Result<()> {
    let perfil = perfil_dos_argumentos(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });

    let armazem = Arc::new(
        match caminho_da_config(
            std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
        ) {
            Some(arquivo) => {
                println!("teclas salvas em {}", arquivo.display());
                ArmazemMapa::com_arquivo(arquivo)
            }
            None => ArmazemMapa::em_memoria(),
        },
    );

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
    println!("perfil: {} ({:04x}:{:04x})", perfil.nome, perfil.fabricante, perfil.produto);
    println!("vpad em http://0.0.0.0:{PORTA}");
    axum::serve(listener, vpad::app_com(tx, perfil_rx, armazem)).await
}
