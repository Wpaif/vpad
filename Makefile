.PHONY: test audit check hooks

# O que precisa passar antes de um merge.
test:
	cargo test
	cd web && npm test
	./scripts/testar-hooks.sh

# Ativa neste clone o bloqueio de commit direto na master (.githooks/pre-commit).
hooks:
	git config core.hooksPath .githooks

# Dependências Rust contra o banco de vulnerabilidades do RustSec.
audit:
	./scripts/auditar.sh

check: test audit
