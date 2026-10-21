use axum::serve::ListenerExt;
use tokio::sync::{mpsc, watch};
use vpad::{
    dispositivo::Gamepad,
    perfil::{indice_do_perfil, perfil_dos_argumentos},
    servico::{self, Dispositivo},
};

const PORTA: u16 = 8080;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let perfil = perfil_dos_argumentos(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });

    let gamepad = Gamepad::novo(perfil).map_err(|e| {
        eprintln!("não consegui criar o gamepad virtual: {e}");
        eprintln!("verifique se o módulo uinput está carregado (sudo modprobe uinput)");
        eprintln!("e se seu usuário tem permissão de escrita em /dev/uinput");
        e
    })?;

    let (tx, rx) = mpsc::unbounded_channel();
    let (perfil_tx, perfil_rx) = watch::channel(indice_do_perfil(perfil.id).unwrap_or(0));
    let fabrica: servico::Fabrica =
        Box::new(|p| Gamepad::novo(p).map(|g| Box::new(g) as Box<dyn Dispositivo>));
    tokio::spawn(servico::processar(rx, perfil_tx, fabrica, Box::new(gamepad)));

    // Sem Nagle: cada toque vira um pacote imediato.
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", PORTA))
        .await?
        .tap_io(|tcp| {
            let _ = tcp.set_nodelay(true);
        });
    println!("perfil: {} ({:04x}:{:04x})", perfil.nome, perfil.fabricante, perfil.produto);
    println!("vpad em http://0.0.0.0:{PORTA}");
    axum::serve(listener, vpad::app_com(tx, perfil_rx)).await
}
