# Contributing to skillctrl

Thank you for your interest in contributing to skillctrl! This document provides guidelines for contributing.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- A code editor (VS Code, IntelliJ IDEA, etc.)

### Building

```bash
# Clone the repository
git clone https://github.com/yourusername/skillctrl.git
cd skillctrl

# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test --workspace
```

### Development Workflow

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Write tests
5. Ensure all tests pass
6. Submit a pull request

## Code Style

We follow standard Rust conventions:

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings
```

## Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p skillctrl-core

# Run tests with output
cargo test --workspace -- --nocapture

# Run specific test
cargo test test_name
```

## Project Structure

```
skillctrl/
├── crates/
│   ├── skillctrl-core/          # Core types and traits
│   │   ├── src/
│   │   │   ├── component.rs     # Component types
│   │   │   ├── endpoint.rs      # Endpoint types
│   │   │   ├── scope.rs         # Scope types
│   │   │   └── ...
│   │   └── Cargo.toml
│   │
│   ├── skillctrl-catalog/       # Catalog/bundle parsing
│   ├── skillctrl-git/           # Git operations
│   ├── skillctrl-state/         # State management
│   │
│   ├── skillctrl-adapter-core/  # Adapter traits
│   ├── skillctrl-adapter-claude/ # Claude Code adapter
│   │
│   ├── skillctrl-importer-core/ # Importer traits
│   ├── skillctrl-importer-claude/ # Claude Code importer
│   │
│   └── skillctrl-cli/           # CLI
│
└── examples/
    └── market/                  # Example catalog and bundles
```

## Adding a New Adapter

To add support for a new AI coding assistant:

1. Create a new crate: `crates/skillctrl-adapter-<name>/`
2. Implement the `Adapter`, `InstallAdapter`, `UninstallAdapter`, and `StatusAdapter` traits
3. Create a corresponding importer crate: `crates/skillctrl-importer-<name>/`
4. Register the adapter in the CLI
5. Add tests and documentation

Example:

```rust
use async_trait::async_trait;
use skillctrl_adapter_core::{Adapter, InstallAdapter, ...};

pub struct MyAdapter;

#[async_trait]
impl Adapter for MyAdapter {
    fn endpoint(&self) -> Endpoint {
        Endpoint::Custom("my-assistant".to_string())
    }

    fn capabilities(&self) -> AdapterCapabilities {
        // ...
    }
}

#[async_trait]
impl InstallAdapter for MyAdapter {
    async fn plan_install(&self, bundle: &BundleManifest, ctx: &InstallContext) -> Result<InstallPlan> {
        // ...
    }

    async fn apply_install(&self, plan: &InstallPlan) -> Result<InstallResult> {
        // ...
    }
}
```

## Adding a New Component Type

1. Add the variant to `ComponentKind` in `skillctrl-core/src/component.rs`
2. Update adapters to handle the new component type
3. Add importers to recognize the component
4. Update documentation

## Documentation

- Public APIs must be documented with rustdoc comments
- User-facing changes should be documented in README.md
- New features should be added to CHANGELOG.md

## Pull Request Guidelines

### PR Title

Use conventional commit format:

```
type(scope): description

Types: feat, fix, docs, style, refactor, test, chore

Examples:
- feat(cli): add dry-run flag to install command
- fix(claude): correct skill directory path
- docs(readme): update installation instructions
```

### PR Description

Include:

- **Why**: What problem does this solve?
- **What**: What changes were made?
- **How**: How was it implemented?
- **Testing**: How was it tested?
- **Breaking Changes**: Are there any breaking changes?

### Review Criteria

- Code follows style guidelines
- Tests are included and passing
- Documentation is updated
- No unnecessary dependencies added
- Backward compatibility maintained (or documented)

## Questions?

Feel free to open an issue for discussion before starting significant work.
