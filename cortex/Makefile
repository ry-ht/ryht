.PHONY: help check build test clean fmt clippy doc run install

help:
	@echo "Cortex - Cognitive Memory System"
	@echo ""
	@echo "Available targets:"
	@echo "  check      - Check code for errors"
	@echo "  build      - Build all crates"
	@echo "  test       - Run all tests"
	@echo "  clean      - Clean build artifacts"
	@echo "  fmt        - Format code"
	@echo "  clippy     - Run clippy linter"
	@echo "  doc        - Generate documentation"
	@echo "  run        - Run the CLI"
	@echo "  install    - Install the CLI binary"

check:
	cargo check --workspace

build:
	cargo build --workspace

release:
	cargo build --release --workspace

test:
	cargo test --workspace

clean:
	cargo clean

fmt:
	cargo fmt --workspace

clippy:
	cargo clippy --workspace -- -D warnings

doc:
	cargo doc --workspace --no-deps --open

run:
	cargo run --bin cortex -- --help

install:
	cargo install --path cortex-cli

# Development helpers
dev-check: fmt clippy test

# Count lines of code
loc:
	@echo "Lines of code:"
	@find . -name "*.rs" -not -path "*/target/*" | xargs wc -l | tail -1
