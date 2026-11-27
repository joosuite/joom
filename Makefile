.DEFAULT_GOAL := build

CARGO ?= $(if $(wildcard $(HOME)/.cargo/bin/cargo),$(HOME)/.cargo/bin/cargo,cargo)
TSX ?= npx --no-install tsx
BIN_DIR := $(CURDIR)/bin
INSTALL_BIN ?= $(if $(CARGO_INSTALL_ROOT),$(CARGO_INSTALL_ROOT),$(if $(CARGO_HOME),$(CARGO_HOME),$(HOME)/.cargo))/bin
EXE := $(if $(filter Windows_NT,$(OS)),.exe,)

.PHONY: build install joom fmt fmt-check lint test tooling-check live-test live-cleanup live-coverage ci audit clean schemas reference

schemas:
	$(TSX) scripts/generate_cli_schemas.ts

reference: schemas
	$(CARGO) build --locked
	$(TSX) scripts/generate_cli_reference.ts
	$(TSX) scripts/generate_wrapper_reference.ts

build:
	$(CARGO) build --release --locked
	@mkdir -p $(BIN_DIR)
	cp target/release/joom$(EXE) $(BIN_DIR)/joom$(EXE)
	$(CARGO) run --release --locked --example install_wrappers -- --directory "$(BIN_DIR)"

install:
	$(CARGO) install --path . --locked
	$(CARGO) run --release --locked --example install_wrappers -- --directory "$(INSTALL_BIN)"

joom:
	$(CARGO) run --locked -- $(ARGS)

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

lint:
	$(CARGO) clippy --locked --all-targets --all-features -- -D warnings

test:
	$(CARGO) test --locked --all-targets --all-features
	npm test

tooling-check:
	npm run check
	npm run check:schemas
	npm run check:wrappers

live-test:
	$(TSX) tests/live/runner.ts

live-cleanup:
	$(TSX) tests/live/runner.ts --cleanup-report "$(JOOM_LIVE_CLEANUP_REPORT)"

live-coverage:
	$(TSX) tests/live/coverage.ts

ci: fmt-check lint test tooling-check

audit:
	$(CARGO) audit --deny warnings

clean:
	$(CARGO) clean
