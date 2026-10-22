#!/bin/sh
# Notas de um release: a seção da versão no CHANGELOG.md, mais como baixar o pacote.
# Uso: ./scripts/notas-do-release.sh 0.1.0   (a versão sem o "v")
set -eu

raiz=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
versao=${1:-}

# a versão vai para um padrão de busca: só o formato x.y.z entra
case "$versao" in
    [0-9]*.[0-9]*.[0-9]*) ;;
    *) echo "uso: notas-do-release.sh <versão, por exemplo 0.1.0>" >&2; exit 1 ;;
esac
if printf '%s' "$versao" | grep -Eqv '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "versão inválida: $versao (esperado x.y.z)" >&2
    exit 1
fi

# da linha "## [versão] - data" até a próxima seção ou até os links do rodapé
secao=$(awk -v v="$versao" '
    index($0, "## [" v "] - ") == 1 { dentro = 1; next }
    dentro && /^## \[/ { exit }
    dentro && /^\[[^]]+\]: / { exit }
    dentro { print }
' "$raiz/CHANGELOG.md" | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}')

if [ -z "$(printf '%s' "$secao" | tr -d '[:space:]')" ]; then
    echo "a versão $versao não está no CHANGELOG.md" >&2
    exit 1
fi

printf '%s\n' "$secao"
cat <<MENSAGEM

---

**Como usar:** baixe o pacote da sua arquitetura, \`vpad-$versao-linux-x86_64.tar.gz\` (Intel/AMD) ou \`vpad-$versao-linux-aarch64.tar.gz\` (ARM de 64 bits, como o Raspberry Pi 4/5 com sistema de 64 bits), confira com o \`.sha256\` ao lado, extraia e rode \`./vpad\`. Ou compile pelo [README](https://github.com/Wpaif/vpad#instalação) (também em [English](https://github.com/Wpaif/vpad/blob/master/README.en.md), [Español](https://github.com/Wpaif/vpad/blob/master/README.es.md), [中文](https://github.com/Wpaif/vpad/blob/master/README.zh-CN.md), [日本語](https://github.com/Wpaif/vpad/blob/master/README.ja.md) e [한국어](https://github.com/Wpaif/vpad/blob/master/README.ko.md)).

Histórico completo: [CHANGELOG.md](https://github.com/Wpaif/vpad/blob/master/CHANGELOG.md).
MENSAGEM
