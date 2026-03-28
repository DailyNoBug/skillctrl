# skillctrl

**Unified skills marketplace for Claude Code, Codex, and Cursor**

skillctrl is a cross-platform tool that lets you manage skills, rules, and components across multiple AI coding assistants from a single unified repository.

## Quick Links

- [📚 Quick Start Guide](QUICKSTART.md) - 快速上手开发指南
- [🏗️ Architecture](ARCHITECTURE.md) - 架构设计文档
- [🤝 Contributing](CONTRIBUTING.md) - 贡献指南
- [📋 Documentation](#documentation) - 详细文档

## Features

- **Unified Content Management**: Maintain one repository of skills, rules, and components
- **Multi-Endpoint Support**: Install to Claude Code, Codex, and Cursor
- **Git-Based Sources**: Pull content from any git repository
- **Import Existing Configs**: Convert your current `.claude`, `.codex`, or `.cursor` configurations
- **Dry Run Mode**: Preview changes before applying them
- **Conflict Resolution**: Smart handling of existing files
- **State Tracking**: Keep track of what's installed where

## Installation

### From Source

```bash
cargo install --locked --path crates/skillctrl-cli
```

### Package A Binary Archive

```bash
bash ./package.sh
```

Windows:

```powershell
pwsh ./scripts/package-windows.ps1
```

### Pre-built Binaries

Coming soon.

## Quick Start

### 1. Add a Source

```bash
skillctrl source add team \
  --repo git@github.com:yourorg/ai-market.git \
  --branch main
```

### 2. List Available Bundles

```bash
skillctrl list --source team
```

### 3. Install a Bundle

```bash
skillctrl install review-pr \
  --source team \
  --target claude-code \
  --scope project \
  --project /path/to/repo
```

### 4. Check Status

```bash
skillctrl status \
  --target claude-code \
  --scope project \
  --project /path/to/repo
```

## Commands

### Source Management

```bash
# Add a new source
skillctrl source add <name> --repo <url> --branch <branch>

# List all sources
skillctrl source list

# Update a source
skillctrl source update <name>

# Remove a source
skillctrl source remove <name>
```

### Bundle Management

```bash
# List available bundles
skillctrl list --source <name>

# Show bundle details
skillctrl show <bundle-id> --source <name>

# Install a bundle
skillctrl install <bundle-id> \
  --source <name> \
  --target <claude-code|codex|cursor> \
  --scope <project|user> \
  --project <path>  # for project scope

# Uninstall a bundle
skillctrl uninstall <bundle-id> \
  --target <claude-code|codex|cursor> \
  --scope <project|user> \
  --project <path>

# Dry run installation
skillctrl install <bundle-id> --source <name> --target claude-code --dry-run
```

### Import

```bash
# Scan for artifacts
skillctrl import scan \
  --from claude-code \
  --path /path/to/repo

# Create import plan
skillctrl import plan \
  --from claude-code \
  --path /path/to/repo \
  --id my-bundle

# Apply import
skillctrl import apply \
  --from claude-code \
  --path /path/to/repo \
  --out ./output
```

### Status

```bash
# Show all installations
skillctrl status --target claude-code --scope user

# Show specific bundle
skillctrl status --target claude-code --scope project --project /path --bundle <id>
```

## Bundle Structure

A bundle is a collection of components with a `bundle.yaml` manifest:

```yaml
apiVersion: skillctrl.dev/v1
kind: Bundle
id: review-pr
name: Review PR
version: 1.2.0
description: Review pull requests with architecture, tests, and security checks
targets:
  - claude-code
  - codex
  - cursor
components:
  - kind: skill
    id: review-pr
    path: components/skills/review-pr
  - kind: rule
    id: review-policy
    path: components/rules/review-policy.md
  - kind: resource
    id: checklist
    path: components/resources/checklist.md
```

## Component Types

| Type | Description | Claude Code | Codex | Cursor |
|------|-------------|-------------|-------|--------|
| `skill` | Reusable AI capability | ✓ | ✓ | ✓ |
| `rule` | Behavior constraint | ✓ | ✓ | ✓ |
| `command` | Slash command | ✓ | ✓ | ✓ |
| `mcp-server` | MCP server | ✓ | ✓ | ✓ |
| `hook` | Lifecycle hook | ✓ | - | - |
| `agent` | AI agent | ✓ | - | - |
| `resource` | Reference material | ✓ | ✓ | - |

## Creating Your Own Bundle

### Directory Structure

```
my-bundle/
  bundle.yaml
  components/
    skills/
      my-skill/
        SKILL.md
    rules/
      my-rule.md
    resources/
      my-resource.md
```

### Example bundle.yaml

```yaml
apiVersion: skillctrl.dev/v1
kind: Bundle
id: my-bundle
name: My Bundle
version: 1.0.0
description: A custom bundle for my workflow
targets:
  - claude-code
components:
  - kind: skill
    id: my-skill
    path: components/skills/my-skill
  - kind: rule
    id: my-rule
    path: components/rules/my-rule.md
```

### Example Skill (SKILL.md)

```markdown
---
description: Help with code reviews
---

# Code Review

You are a code review assistant. When reviewing code:

1. Check for common bugs and anti-patterns
2. Verify error handling
3. Look for security issues
4. Suggest improvements
```

## Development

### Building

```bash
cargo build --locked --release
```

### Packaging

```bash
bash ./package.sh
```

Packaged archives are written to `dist/`.

### Running Tests

```bash
cargo test --workspace --locked
```

### Releases

Push a tag like `v0.0.1` to trigger GitHub Actions to build and publish macOS, Linux, and Windows release archives to GitHub Releases.

### Project Structure

```
skillctrl/
├── crates/
│   ├── skillctrl-core/        # Core types and traits
│   ├── skillctrl-catalog/     # Catalog/bundle parsing
│   ├── skillctrl-git/         # Git operations
│   ├── skillctrl-state/       # State management
│   ├── skillctrl-adapter-core/     # Adapter traits
│   ├── skillctrl-adapter-claude/   # Claude Code adapter
│   ├── skillctrl-adapter-codex/    # Codex adapter
│   ├── skillctrl-adapter-cursor/   # Cursor adapter
│   ├── skillctrl-importer-core/    # Importer traits
│   ├── skillctrl-importer-claude/  # Claude Code importer
│   ├── skillctrl-exporter-core/    # Exporter traits
│   └── skillctrl-cli/         # CLI
└── examples/
    └── market/                # Example catalog and bundles
```

## Implementation Status

| Feature | Status |
|---------|--------|
| Core types and traits | ✅ |
| Catalog/bundle parsing | ✅ |
| Git operations | ✅ |
| State management (SQLite) | ✅ |
| Claude Code adapter | ✅ |
| Claude Code importer | ✅ |
| Codex adapter | ✅ |
| Cursor adapter | ✅ |
| Native export framework | ✅ |
| TUI interface | ⏳ Planned |
| Plugin system | ⏳ Planned |

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## License

MIT OR Apache-2.0

## Acknowledgments

Built with:
- [Rust](https://www.rust-lang.org/)
- [clap](https://github.com/clap-rs/clap) - CLI parsing
- [tokio](https://tokio.rs/) - Async runtime
- [git2](https://github.com/rust-lang/git2-rs) - Git operations
