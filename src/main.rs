use axum::serve::ListenerExt;
use tokio::sync::mpsc;

const PORTA: u16 = 8080;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel();
    // O gamepad virtual (uinput) vai consumir estes eventos; por ora só registra.
    tokio::spawn(async move {
        while let Some(evento) = rx.recv().await {
            println!("{evento:?}");
        }
    });

    // Sem Nagle: cada toque vira um pacote imediato.
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", PORTA))
        .await?
        .tap_io(|tcp| {
            let _ = tcp.set_nodelay(true);
        });
    println!("vpad em http://0.0.0.0:{PORTA}");
    axum::serve(listener, vpad::app_com(tx)).await
}
