.PHONY: build

help:
	@echo "Othello Commands:"
	@echo ""
	@echo "Building:"
	@echo "  make build       - Build debug version"
	@echo "  make release     - Build optimized release"
	@echo ""
	@echo "Running:"
	@echo "  make run         - Run debug version"
	@echo "  make run-release - Run release version"
	@echo ""
	@echo "Quality:"
	@echo "  make test        - Run tests"
	@echo "  make fmt         - Format code"
	@echo "  make lint        - Run clippy"
	@echo "  make check       - Run all checks"
	@echo ""
	@echo "Maintenance:"
	@echo "  make clean       - Remove build artifacts"

build:
	cargo build

release:
	cargo build --release

run:
	cargo run

run-release:
	cargo run --release

test:
	cargo test

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

check: fmt lint test

clean:
	cargo clean

all: check build
