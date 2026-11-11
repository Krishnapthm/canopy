# Every command an agent or a human needs. See AGENTS.md for when to run which.
CARGO ?= cargo

.PHONY: check test e2e bench gen web build check-docs

check: web
	@if rg -n '\p{Emoji_Presentation}|\x{FE0F}' crates/canopy/src/tui web/src; then echo "emoji code points are forbidden"; exit 1; fi
	@if rg -n --pcre2 "@lobehub/icons(?!/es/[A-Za-z0-9]+/components/Mono[.]js[\"'])" web/src; then echo "use per-brand @lobehub/icons Mono.js imports (ADR 0027)"; exit 1; fi
	$(CARGO) fmt --all --check
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	cd web && npx tsc --noEmit

web/node_modules: web/package.json
	cd web && npm ci --silent
	@touch $@

test: web
	python3 scripts/record-fixture.py --test
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
