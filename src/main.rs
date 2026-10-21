const PORTA: u16 = 8080;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", PORTA)).await?;
    println!("vpad em http://0.0.0.0:{PORTA}");
    axum::serve(listener, vpad::app()).await
}
