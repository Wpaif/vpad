.PHONY: test audit check hooks

# O que precisa passar antes de um merge.
test:
	cargo test
	cd web && npm test
	./scripts/testar-hooks.sh

# Instala neste clone o bloqueio de commit direto na master (.githooks/pre-commit -> .git/hooks).
hooks:
	./scripts/instalar-hooks.sh

# Dependências Rust contra o banco de vulnerabilidades do RustSec.
audit:
	./scripts/auditar.sh

check: test audit
