use std::{fs, time::Duration};

use vpad::{acesso::Acesso, dispositivos::Dispositivos, pareamento::Pareamento};

/// Revogar em outro processo (`vpad --revogar`) tem de valer no servidor na hora, sem
/// esperar a releitura periódica de 2 s.
#[tokio::test]
async fn revogacao_de_outro_processo_vale_na_hora() {
    let pasta = std::env::temp_dir().join(format!(
        "vpad-revogacao-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    fs::create_dir_all(&pasta).unwrap();
    let arquivo = pasta.join("dispositivos.json");

    let dispositivos = Dispositivos::com_arquivo(arquivo.clone());
    let p = dispositivos.parear("iPhone", 0).unwrap();
    let acesso = Acesso::novo(
        dispositivos,
        Pareamento::com_segredo(b"segredo-de-teste-123".to_vec()),
        Duration::from_secs(6),
    );
    assert!(acesso.autorizado(&p.token));

    let observador = vpad::vigiar_revogacoes(acesso.clone()).unwrap().expect("cadastro em disco");

    // outro processo abre o mesmo arquivo e remove o aparelho
    let outro = Dispositivos::com_arquivo(arquivo);
    assert!(outro.revogar(&p.id).unwrap());

    for _ in 0..200 {
        if !acesso.autorizado(&p.token) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(!acesso.autorizado(&p.token), "o inotify não recarregou o cadastro");

    observador.abort();
    fs::remove_dir_all(&pasta).ok();
}
