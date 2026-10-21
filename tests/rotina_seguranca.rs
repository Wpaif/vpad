//! A rotina de auditoria é um script. Estes testes usam um `cargo` falso para verificar que
//! ele roda o `cargo audit` de verdade, propaga a falha e explica como instalar a ferramenta.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

struct Pasta(PathBuf);

impl Pasta {
    fn nova(nome: &str) -> Self {
        let pasta = std::env::temp_dir().join(format!("vpad-rotina-{}-{nome}", std::process::id()));
        let _ = fs::remove_dir_all(&pasta);
        fs::create_dir_all(&pasta).unwrap();
        Self(pasta)
    }

    fn caminho(&self) -> &Path {
        &self.0
    }
}

impl Drop for Pasta {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Cria um `cargo` de mentira que anota as chamadas em `log` e se comporta como `corpo` manda.
fn cargo_falso(pasta: &Pasta, corpo: &str) -> PathBuf {
    let caminho = pasta.caminho().join("cargo");
    fs::write(&caminho, format!("#!/bin/sh\n{corpo}\n")).unwrap();
    fs::set_permissions(&caminho, fs::Permissions::from_mode(0o755)).unwrap();
    pasta.caminho().join("log")
}

fn rodar(pasta: &Pasta, log: &Path) -> Output {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/auditar.sh");
    Command::new("sh")
        .arg(script)
        .env("PATH", format!("{}:/usr/bin:/bin", pasta.caminho().display()))
        .env("VPAD_LOG", log)
        .output()
        .unwrap()
}

#[test]
fn roda_o_cargo_audit() {
    let pasta = Pasta::nova("ok");
    let log = cargo_falso(
        &pasta,
        r#"echo "$@" >> "$VPAD_LOG"
if [ "$1" = "audit" ] && [ "$2" = "--version" ]; then exit 0; fi
exit 0"#,
    );

    let saida = rodar(&pasta, &log);

    assert!(saida.status.success(), "{saida:?}");
    let chamadas = fs::read_to_string(&log).unwrap();
    assert!(chamadas.lines().any(|l| l == "audit --version"), "{chamadas}");
    assert!(chamadas.lines().any(|l| l == "audit"), "{chamadas}");
    assert!(String::from_utf8_lossy(&saida.stdout).to_lowercase().contains("rustsec"));
}

#[test]
fn falha_quando_o_cargo_audit_encontra_problema() {
    let pasta = Pasta::nova("falha");
    let log = cargo_falso(
        &pasta,
        r#"echo "$@" >> "$VPAD_LOG"
if [ "$1" = "audit" ] && [ "$2" = "--version" ]; then exit 0; fi
exit 7"#,
    );

    let saida = rodar(&pasta, &log);

    assert_eq!(saida.status.code(), Some(7), "a falha da auditoria tem de chegar à rotina");
}

#[test]
fn explica_como_instalar_quando_falta_o_cargo_audit() {
    let pasta = Pasta::nova("sem-ferramenta");
    let log = cargo_falso(&pasta, r#"echo "$@" >> "$VPAD_LOG"
exit 101"#);

    let saida = rodar(&pasta, &log);

    assert!(!saida.status.success());
    let erro = String::from_utf8_lossy(&saida.stderr);
    assert!(erro.contains("cargo install cargo-audit"), "{erro}");
}
