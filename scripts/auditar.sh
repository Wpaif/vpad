#!/bin/sh
# Auditoria das dependências Rust contra o banco de vulnerabilidades do RustSec.
# Rode antes de cada merge e de vez em quando: ./scripts/auditar.sh (ou make audit).
set -eu

raiz=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$raiz"

if ! cargo audit --version >/dev/null 2>&1; then
    echo "cargo-audit não está instalado. Instale com:" >&2
    echo "  cargo install cargo-audit --locked" >&2
    exit 2
fi

echo "auditando dependências Rust contra o banco do RustSec..."
cargo audit "$@"
