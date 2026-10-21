use axum::serve::ListenerExt;
use tokio::sync::mpsc;
use vpad::{dispositivo::Gamepad, perfil::perfil_dos_argumentos};

const PORTA: u16 = 8080;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let perfil = perfil_dos_argumentos(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });

    let mut gamepad = Gamepad::novo(perfil).map_err(|e| {
        eprintln!("não consegui criar o gamepad virtual: {e}");
        eprintln!("verifique se o módulo uinput está carregado (sudo modprobe uinput)");
        eprintln!("e se seu usuário tem permissão de escrita em /dev/uinput");
        e
    })?;

    let (tx, mut rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(evento) = rx.recv().await {
            if let Err(e) = gamepad.aplicar(evento) {
                eprintln!("erro ao enviar {evento:?} ao uinput: {e}");
            }
        }
    });

    // Sem Nagle: cada toque vira um pacote imediato.
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", PORTA))
        .await?
        .tap_io(|tcp| {
            let _ = tcp.set_nodelay(true);
        });
    println!("perfil: {} ({:04x}:{:04x})", perfil.nome, perfil.fabricante, perfil.produto);
    println!("vpad em http://0.0.0.0:{PORTA}");
    axum::serve(listener, vpad::app_com(tx)).await
}
