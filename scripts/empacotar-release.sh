#!/bin/sh
# Empacota o binário de release: vpad-<versão>-linux-<arquitetura>.tar.gz (+ .sha256) com o binário,
# a licença, o changelog e os README. Recusa uma versão diferente da do Cargo.toml e um binário que
# não seja um ELF da arquitetura pedida (evita publicar o binário de uma arquitetura no pacote da outra).
# Uso: ./scripts/empacotar-release.sh <versão> <binário> <pasta-de-saída> [x86_64|aarch64]
# Imprime o caminho do .tar.gz.
set -eu

if [ "$#" -lt 3 ] || [ "$#" -gt 4 ]; then
    echo "uso: empacotar-release.sh <versão> <binário> <pasta-de-saída> [x86_64|aarch64]" >&2
    exit 1
fi
versao=$1
binario=$2
destino=$3
arq=${4-x86_64}
raiz=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

# o campo e_machine do cabeçalho ELF (2 bytes, little endian) de cada arquitetura
case "$arq" in
    x86_64) esperado="3e00" ;;
    aarch64) esperado="b700" ;;
    *)
        echo "arquitetura desconhecida: use x86_64 ou aarch64" >&2
        exit 1
        ;;
esac

atual=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$raiz/Cargo.toml" | head -n 1)
if [ "$versao" != "$atual" ]; then
    echo "a versão pedida ($versao) é diferente da do Cargo.toml ($atual)" >&2
    exit 1
fi
if [ ! -f "$binario" ] || [ ! -x "$binario" ]; then
    echo "binário não encontrado ou sem permissão de execução: $binario" >&2
    exit 1
fi

# cabeçalho ELF: magia nos bytes 0-3 e a máquina nos bytes 18-19
cabecalho=$(od -An -v -tx1 -N20 "$binario" | tr -s ' \n' ' ')
# shellcheck disable=SC2086
set -- $cabecalho
if [ "$#" -lt 20 ] || [ "$1$2$3$4" != "7f454c46" ]; then
    echo "o arquivo não é um binário ELF: $binario" >&2
    exit 1
fi
real="${19}${20}"
if [ "$real" != "$esperado" ]; then
    case "$real" in
        3e00) achado="x86_64" ;;
        b700) achado="aarch64" ;;
        *) achado="outra ($real)" ;;
    esac
    echo "o binário é $achado, mas a arquitetura pedida é $arq" >&2
    exit 1
fi

nome="vpad-$versao-linux-$arq"
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
