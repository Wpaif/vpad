.PHONY: test audit check

# O que precisa passar antes de um merge.
test:
	cargo test
	cd web && npm test

# Dependências Rust contra o banco de vulnerabilidades do RustSec.
audit:
	./scripts/auditar.sh

check: test audit
