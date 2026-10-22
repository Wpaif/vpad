#!/bin/sh
# A versão a empacotar: de uma tag vX.Y.Z, X.Y.Z; de qualquer outra coisa (nome de branch, vazio),
# a versão do Cargo.toml. Assim o teste sem tag do workflow de release usa uma versão válida.
# Uso: ./scripts/versao-do-release.sh <tag ou nome da ref>
set -eu

raiz=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
nome=${1:-}

if printf '%s' "$nome" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
    printf '%s\n' "${nome#v}"
else
    sed -n 's/^version = "\(.*\)"$/\1/p' "$raiz/Cargo.toml" | head -n 1
fi
