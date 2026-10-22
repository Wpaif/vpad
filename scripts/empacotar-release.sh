#!/bin/sh
# Empacota o binário de release: vpad-<versão>-linux-x86_64.tar.gz (+ .sha256) com o binário, a
# licença, o changelog e os README. Recusa uma versão diferente da do Cargo.toml.
# Uso: ./scripts/empacotar-release.sh <versão> <binário> <pasta-de-saída>
# Imprime o caminho do .tar.gz.
set -eu

[ "$#" -eq 3 ] || { echo "uso: empacotar-release.sh <versão> <binário> <pasta-de-saída>" >&2; exit 1; }
versao=$1
binario=$2
destino=$3
raiz=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

atual=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$raiz/Cargo.toml" | head -n 1)
if [ "$versao" != "$atual" ]; then
    echo "a versão pedida ($versao) é diferente da do Cargo.toml ($atual)" >&2
    exit 1
fi
if [ ! -f "$binario" ] || [ ! -x "$binario" ]; then
    echo "binário não encontrado ou sem permissão de execução: $binario" >&2
    exit 1
fi

nome="vpad-$versao-linux-x86_64"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/$nome"
cp "$binario" "$tmp/$nome/vpad"
chmod 755 "$tmp/$nome/vpad"
cp "$raiz"/README*.md "$raiz/LICENSE" "$raiz/CHANGELOG.md" "$tmp/$nome/"

mkdir -p "$destino"
# dono 0/0: o pacote não carrega o usuário de quem o gerou
tar --owner=0 --group=0 --numeric-owner -C "$tmp" -czf "$destino/$nome.tar.gz" "$nome"
(cd "$destino" && sha256sum "$nome.tar.gz" > "$nome.tar.gz.sha256")

echo "$destino/$nome.tar.gz"
