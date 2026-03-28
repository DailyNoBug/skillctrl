# skillctrl 快速上手开发指南

## 项目概述

`skillctrl` 是一个用 Rust 编写的跨平台工具，用于统一管理 Claude Code、Codex 和 Cursor 的 skills、rules 和组件。

## 环境准备

### 1. 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. 克隆项目

```bash
git clone https://github.com/yourusername/skillctrl.git
cd skillctrl
```

### 3. 验证环境

```bash
rustc --version    # 应显示 1.70+
cargo --version    # 应显示最新版本
```

## 项目结构

```
skillctrl/
├── crates/                    # 所有 Rust crates
│   ├── skillctrl-core/         # 核心类型和 trait
│   ├── skillctrl-catalog/      # Catalog/bundle 解析
│   ├── skillctrl-git/          # Git 操作
│   ├── skillctrl-state/        # SQLite 状态管理
│   ├── skillctrl-adapter-core/ # Adapter trait 定义
│   ├── skillctrl-adapter-claude/   # Claude Code adapter
│   ├── skillctrl-adapter-codex/    # Codex adapter
│   ├── skillctrl-adapter-cursor/   # Cursor adapter
│   ├── skillctrl-importer-core/    # Importer trait 定义
│   ├── skillctrl-importer-claude/  # Claude Code importer
│   ├── skillctrl-exporter-core/    # Exporter trait 定义
│   └── skillctrl-cli/          # CLI 入口
├── examples/
│   └── market/                 # 示例 catalog 和 bundles
├── Cargo.toml                 # Workspace 配置
├── Makefile                   # 构建工具
└── build.sh                   # 构建脚本
```

## 快速开始

### 1. 构建项目

```bash
# 使用 Makefile
make build

# 或使用 cargo
cargo build --locked --release

# 或使用脚本
./build.sh
```

构建完成后，二进制文件位于：`target/release/skillctrl`

如果要一键打包可分发归档：

```bash
make package

# 或
bash ./package.sh
```

打包产物会输出到：`dist/`

### 2. 运行测试

```bash
# 运行所有测试
make test

# 或
cargo test --workspace --locked
```

### 3. 安装到本地

```bash
make install

# 或
cargo install --locked --path crates/skillctrl-cli
```

## 开发工作流

### 添加新的命令

1. 在 `crates/skillctrl-cli/src/main.rs` 的 `Commands` enum 中添加新命令
2. 实现对应的处理函数
3. 测试

```rust
// 1. 添加命令变体
enum Commands {
    // ...
    MyNewCommand {
        arg1: String,
        arg2: Option<String>,
    },
}

// 2. 添加处理分支
async fn run_command(cli: Cli) -> Result<()> {
    match cli.command {
        // ...
        Commands::MyNewCommand { arg1, arg2 } => {
            handle_my_new_command(arg1, arg2).await
        }
    }
}

// 3. 实现处理函数
async fn handle_my_new_command(arg1: String, arg2: Option<String>) -> Result<()> {
    println!("执行新命令: {}", arg1);
    Ok(())
}
```

### 添加新的 Adapter

1. 创建新的 adapter crate
2. 实现 `Adapter`、`InstallAdapter` 等 trait
3. 在 CLI 中注册

```bash
# 创建目录
mkdir -p crates/skillctrl-adapter-myai/src

# 创建 Cargo.toml
cat > crates/skillctrl-adapter-myai/Cargo.toml << 'EOF'
[package]
name = "skillctrl-adapter-myai"
version.workspace = true
edition.workspace = true

[dependencies]
skillctrl-core = { path = "../skillctrl-core" }
skillctrl-adapter-core = { path = "../skillctrl-adapter-core" }
async-trait = "0.1"
anyhow = { workspace = true }
EOF

# 添加到 workspace members
# 编辑 Cargo.toml
```

最小 adapter 实现：

```rust
use async_trait::async_trait;
use skillctrl_adapter_core::*;
use skillctrl_core::*;

pub struct MyAIAdapter;

#[async_trait]
impl Adapter for MyAIAdapter {
    fn endpoint(&self) -> Endpoint {
        Endpoint::Custom("my-ai".to_string())
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities::default()
    }
}

#[async_trait]
impl InstallAdapter for MyAIAdapter {
    async fn plan_install(&self, bundle: &BundleManifest, ctx: &InstallContext) -> Result<InstallPlan> {
        // 实现安装计划
    }

    async fn apply_install(&self, plan: &InstallPlan) -> Result<InstallResult> {
        // 实现安装
    }
}
```

### 添加新的组件类型

1. 在 `crates/skillctrl-core/src/component.rs` 中添加到 `ComponentKind`
2. 更新各 adapter 以处理新类型

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentKind {
    // ... 现有类型
    MyNewType,
}
```

## 调试技巧

### 启用详细日志

```bash
RUST_LOG=debug skillctrl --verbose install my-bundle --source team
```

### 在 VS Code 中调试

创建 `.vscode/launch.json`:

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug skillctrl",
            "cargo": {
                "args": [
                    "build",
                    "--package=skillctrl"
                ],
                "filter": {
                    "name": "skillctrl",
                    "kind": "bin"
                }
            },
            "args": [],
            "cwd": "${workspaceFolder}"
        }
    ]
}
```

### 使用单元测试

```bash
# 运行特定 crate 的测试
cargo test -p skillctrl-core

# 运行特定测试
cargo test test_component_kind

# 输出测试信息
cargo test -- --nocapture
```

## 常见任务

### 添加依赖

```bash
# 添加到 workspace
cargo add serde_yaml --workspace

# 添加到特定 crate
cargo add --package skillctrl-core regex
```

### 代码格式化

```bash
make fmt
# 或
cargo fmt
```

### 代码检查

```bash
make clippy
# 或
cargo clippy -- -D warnings
```

### 清理构建

```bash
make clean
# 或
cargo clean
```

## 核心概念

### Component（组件）

可安装的最小单元，包括：
- `skill` - AI 能力
- `rule` - 行为约束
- `command` - 命令
- `mcp-server` - MCP 服务器
- `hook` - 生命周期钩子
- `resource` - 资源文件
- `agent` - AI 代理

### Bundle（包）

一组组件的集合，通过 `bundle.yaml` 描述：

```yaml
apiVersion: skillctrl.dev/v1
kind: Bundle
id: my-bundle
name: My Bundle
version: 1.0.0
components:
  - kind: skill
    id: my-skill
    path: components/skills/my-skill
```

### Catalog（目录）

多个 bundle 的集合，通过 `catalog.yaml` 描述：

```yaml
apiVersion: skillctrl.dev/v1
kind: Catalog
name: my-catalog
bundles:
  - id: my-bundle
    version: 1.0.0
    path: bundles/my-bundle
```

### Adapter（适配器）

负责将组件安装到特定端点的实现。每个 AI 软件需要一个 adapter。

### Importer（导入器）

负责从现有配置扫描并转换为 bundle 格式。

## 性能优化

### 并行处理

使用 `tokio` 的异步特性：

```rust
use futures::future::join_all;

let tasks = sources.iter()
    .map(|s| self.clone().fetch(s))
    .collect::<Vec<_>>();

let results = join_all(tasks).await;
```

### 缓存

使用 `skillctrl-cache` crate（待实现）缓存 bundle 解析结果。

### 数据库

使用 SQLite 索引加速查询：
```rust
state.query_installations(Some(bundle_id), Some(&endpoint), Some(scope)).await?
```

## 扩展建议

### 短期（已实现）

- ✅ Claude Code adapter
- ✅ Codex adapter
- ✅ Cursor adapter
- ✅ 导出功能框架

### 中期

- ⏳ TUI 界面（使用 `ratatui`）
- ⏳ 插件系统（动态加载 adapter）
- ⏳ 中间件系统
- ⏳ 事件系统

### 长期

- ⏳ GUI 桌面应用（使用 `tauri`）
- ⏳ Web API（使用 `axum`）
- ⏳ 云端同步
- ⏳ 团队协作功能

## 贡献流程

1. Fork 项目
2. 创建功能分支：`git checkout -b feat/my-feature`
3. 提交：`git commit -m "feat: add my feature"`
4. 推送：`git push origin feat/my-feature`
5. 创建 Pull Request

### Commit 规范

```
type(scope): description

Types:
- feat: 新功能
- fix: 修复
- docs: 文档
- style: 格式
- refactor: 重构
- test: 测试
- chore: 构建/工具
```

## 有用的链接

- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio 教程](https://tokio.rs/tokio/tutorial)
- [Clap 文档](https://docs.rs/clap/)
- [项目 README](./README.md)
- [贡献指南](./CONTRIBUTING.md)

## 故障排除

### 构建错误

```bash
# 清理后重新构建
cargo clean && cargo build --release

# 更新依赖
cargo update
```

### 测试失败

```bash
# 运行特定测试
cargo test test_name -- --exact

# 显示测试输出
cargo test -- --nocapture --test-threads=1
```

### 数据库错误

```bash
# 删除状态数据库
rm ~/.config/skillctrl/state.db

# 重新初始化
skillctrl status
```

## 获取帮助

- GitHub Issues: [问题追踪](https://github.com/yourusername/skillctrl/issues)
- Discussions: [讨论区](https://github.com/yourusername/skillctrl/discussions)
- Email: support@skillctrl.dev
