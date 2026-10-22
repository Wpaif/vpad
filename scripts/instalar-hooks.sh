#!/bin/sh
# Instala o hook pre-commit (bloqueio de commit direto na master) COPIANDO-O para .git/hooks.
# Cópia, e não core.hooksPath: .git/hooks existe em qualquer checkout, e um hooksPath que aponta
# para uma pasta ausente (uma branch antiga, por exemplo) deixaria o repositório sem proteção em silêncio.
# Uso: ./scripts/instalar-hooks.sh [pasta-do-repositorio]   (ou: make hooks)
set -eu

aqui=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
alvo=${1:-$aqui}

if ! git -C "$alvo" rev-parse --git-dir >/dev/null 2>&1; then
    echo "não é um repositório git: $alvo" >&2
    exit 1
fi

comum=$(git -C "$alvo" rev-parse --path-format=absolute --git-common-dir)
mkdir -p "$comum/hooks"
cp "$aqui/.githooks/pre-commit" "$comum/hooks/pre-commit"
chmod +x "$comum/hooks/pre-commit"
# um core.hooksPath faria o git ignorar .git/hooks
git -C "$alvo" config --unset core.hooksPath 2>/dev/null || true

echo "hook instalado: $comum/hooks/pre-commit"
