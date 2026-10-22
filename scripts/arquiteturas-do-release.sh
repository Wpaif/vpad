#!/bin/sh
# A matriz de arquiteturas do workflow de release, em JSON numa linha só. Cada arquitetura compila
# NATIVAMENTE no seu runner (sem compilação cruzada, que complicaria as dependências com código C).
# Uso: ./scripts/arquiteturas-do-release.sh [todas|x86_64|aarch64]   (sem argumento: todas)
set -eu

entrada=${1:-}
x86='{"arch":"x86_64","runner":"ubuntu-22.04"}'
arm='{"arch":"aarch64","runner":"ubuntu-22.04-arm"}'

case "$entrada" in
    "" | todas) printf '{"include":[%s,%s]}\n' "$x86" "$arm" ;;
    x86_64) printf '{"include":[%s]}\n' "$x86" ;;
    aarch64) printf '{"include":[%s]}\n' "$arm" ;;
    *)
        echo "arquitetura desconhecida (use todas, x86_64 ou aarch64)" >&2
        exit 1
        ;;
esac
