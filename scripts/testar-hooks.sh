#!/bin/sh
# Testa o hook .githooks/pre-commit num repositório temporário. Uso: ./scripts/testar-hooks.sh
set -u

raiz=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
falhas=0
total=0

ok() { total=$((total + 1)); printf '  ok   %s\n' "$1"; }
falhou() { total=$((total + 1)); falhas=$((falhas + 1)); printf '  FALHOU %s\n' "$1"; }
# espera sucesso (esperado=0) ou falha (esperado=1) do comando dado
verifica() {
  nome=$1; esperado=$2; shift 2
  if "$@" >"$tmp/saida" 2>&1; then real=0; else real=1; fi
  if [ "$real" = "$esperado" ]; then ok "$nome"; else falhou "$nome"; sed 's/^/       | /' "$tmp/saida"; fi
}

novo_repo() {
  rm -rf "$tmp/r"; mkdir "$tmp/r"; cd "$tmp/r" || exit 1
  git init -q -b master
  git config user.name teste; git config user.email teste@example.com
  mkdir -p .githooks
  [ -f "$raiz/.githooks/pre-commit" ] && cp "$raiz/.githooks/pre-commit" .githooks/pre-commit
  git config core.hooksPath .githooks
  echo base > a.txt; git add a.txt
  git add .githooks 2>/dev/null || true
}

echo "hook pre-commit:"

novo_repo
verifica "bloqueia o commit direto na master" 1 git commit -q -m "direto"
git commit -q -m "direto" 2>"$tmp/msg"; grep -qi "master" "$tmp/msg" && ok "a mensagem explica o bloqueio e cita a master" || falhou "a mensagem explica o bloqueio e cita a master"
git commit -q -m "direto" 2>"$tmp/msg"; grep -q "VPAD_PERMITIR_COMMIT_NA_MASTER" "$tmp/msg" && ok "a mensagem diz como liberar em emergência" || falhou "a mensagem diz como liberar em emergência"
verifica "nada foi commitado quando bloqueou" 1 git rev-parse -q --verify HEAD

novo_repo
verifica "libera com VPAD_PERMITIR_COMMIT_NA_MASTER=1" 0 env VPAD_PERMITIR_COMMIT_NA_MASTER=1 git commit -q -m "emergência"
novo_repo
verifica "o valor da variável precisa ser exatamente 1" 1 env VPAD_PERMITIR_COMMIT_NA_MASTER=sim git commit -q -m "x"
novo_repo
verifica "--no-verify continua valendo (é do git)" 0 git commit -q --no-verify -m "x"

novo_repo
VPAD_PERMITIR_COMMIT_NA_MASTER=1 git commit -q -m base
git checkout -q -b feature/x
echo f > f.txt; git add f.txt
verifica "permite commit numa branch de funcionalidade" 0 git commit -q -m "feature"
git checkout -q master
verifica "merge --no-ff na master não é barrado" 0 git merge --no-ff -q feature/x -m "merge"
verifica "o merge criou o commit de merge" 0 sh -c '[ "$(git log -1 --format=%p | wc -w)" = 2 ]'

novo_repo
VPAD_PERMITIR_COMMIT_NA_MASTER=1 git commit -q -m base
git checkout -q -b feature/y; echo y > y.txt; git add y.txt; git commit -q -m y; git checkout -q master
verifica "merge fast-forward na master não é barrado" 0 git merge --ff-only -q feature/y

# conflito: concluir o merge à mão (MERGE_HEAD existe) é permitido
novo_repo
VPAD_PERMITIR_COMMIT_NA_MASTER=1 git commit -q -m base
git checkout -q -b feature/c; echo da-feature > a.txt; git commit -q -am feature
git checkout -q master; echo da-master > a.txt; VPAD_PERMITIR_COMMIT_NA_MASTER=1 git commit -q -am master
git merge --no-ff -q feature/c >/dev/null 2>&1
echo resolvido > a.txt; git add a.txt
verifica "concluir um merge com conflito resolvido é permitido" 0 git commit -q --no-edit

novo_repo
VPAD_PERMITIR_COMMIT_NA_MASTER=1 git commit -q -m base
git checkout -q --detach
echo d > d.txt; git add d.txt
verifica "permite commit com HEAD solto (detached)" 0 git commit -q -m detached

echo
if [ "$falhas" -eq 0 ]; then echo "tudo certo: $total verificações"; exit 0; fi
echo "$falhas de $total verificações falharam"; exit 1
