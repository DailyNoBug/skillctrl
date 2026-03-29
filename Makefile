.PHONY: all build test clean install fmt clippy run package

CARGO ?= $(shell command -v cargo 2>/dev/null || echo "$(HOME)/.cargo/bin/cargo")

# Default target
all: build

# Build the project
build:
	@echo "Building skillctrl-desktop frontend and native binaries..."
	@test -x "$(CARGO)" || (echo "cargo not found. Install Rust: https://rustup.rs and run 'source $$HOME/.cargo/env'" && exit 1)
	@if [ ! -d "skillctrl-desktop/node_modules" ]; then cd skillctrl-desktop && npm ci --no-fund --no-audit; fi
	cd skillctrl-desktop && npm run build
	$(CARGO) build --locked --release

# Build in debug mode
debug:
	@echo "Building skillctrl-desktop frontend and debug binaries..."
	@test -x "$(CARGO)" || (echo "cargo not found. Install Rust: https://rustup.rs and run 'source $$HOME/.cargo/env'" && exit 1)
	@if [ ! -d "skillctrl-desktop/node_modules" ]; then cd skillctrl-desktop && npm ci --no-fund --no-audit; fi
	cd skillctrl-desktop && npm run build
	$(CARGO) build --locked

# Run tests
test:
	@echo "Running tests..."
	@if [ ! -d "skillctrl-desktop/node_modules" ]; then cd skillctrl-desktop && npm ci --no-fund --no-audit; fi
	cd skillctrl-desktop && npm run build
	$(CARGO) test --workspace --locked

# Run tests with output
test-verbose:
	@echo "Running tests (verbose)..."
	@if [ ! -d "skillctrl-desktop/node_modules" ]; then cd skillctrl-desktop && npm ci --no-fund --no-audit; fi
	cd skillctrl-desktop && npm run build
	$(CARGO) test --workspace --locked -- --nocapture

# Format code
fmt:
	@echo "Formatting code..."
	$(CARGO) fmt

# Run linter
clippy:
	@echo "Running clippy..."
	$(CARGO) clippy --locked -- -D warnings

# Clean build artifacts
clean:
	@echo "Cleaning..."
	$(CARGO) clean

# Install locally
install: build
	@echo "Installing skillctrl..."
	$(CARGO) install --locked --path crates/skillctrl-cli

# Package a distributable binary archive
package:
	@echo "Packaging skillctrl and skillctrl-desktop..."
	bash ./package.sh

# Run the CLI
run:
	$(CARGO) run --locked -- --help

# Update dependencies
update:
	@echo "Updating dependencies..."
	$(CARGO) update

# Check for issues (without building)
check:
	@echo "Checking..."
	@if [ ! -d "skillctrl-desktop/node_modules" ]; then cd skillctrl-desktop && npm ci --no-fund --no-audit; fi
	cd skillctrl-desktop && npm run build
	$(CARGO) check --workspace --locked

# Run all checks
check-all: fmt clippy test
	@echo "All checks passed!"

# Run example commands
demo:
	@echo "Running demo..."
	./target/release/skillctrl source list || true
