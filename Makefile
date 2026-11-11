# Every command an agent or a human needs. See AGENTS.md for when to run which.
CARGO ?= cargo

.PHONY: check test e2e bench gen web build check-docs

check: web
	$(CARGO) fmt --all --check
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	cd web && npx tsc --noEmit

web/node_modules: web/package.json
	cd web && npm ci --silent
	@touch $@

test: web
	$(CARGO) test --workspace --lib --bins
	cd web && node check.mjs

e2e: web
	$(CARGO) test -p canopy --test e2e

bench:
	$(CARGO) test --release -p canopy --test bench -- --nocapture --test-threads=1

gen:
	CANOPY_GEN_OUT=$(CURDIR)/web/src/protocol.gen.ts $(CARGO) test -q -p protocol --features ts-rs write_typescript

web: web/node_modules
	cd web && npm run build

build: web
	$(CARGO) build --release

check-docs:
	./scripts/check-docs.sh
