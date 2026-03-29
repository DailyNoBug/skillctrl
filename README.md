# skillctrl

`skillctrl` 是一个统一管理 AI 编码助手资产的命令行工具。它帮助你把同一份资产仓库中的 `skill`、`rule`、`mcp`、`resource`、`agent`、`command`、`hook` 等内容，安装到 Claude Code、Codex、Cursor 等目标环境中。

## 项目定位

这个仓库主要解决几个问题：

- 用一套 source 仓库统一维护多种 AI 助手资产
- 用统一 CLI 管理安装、更新、校验和导出
- 把项目级和用户级安装状态记录下来，方便追踪和自动化
- 给脚本和上层系统提供稳定的 `--json-resp` 输出

## 核心能力

- Source 管理：支持本地目录、SSH 仓库、HTTPS 仓库
- 资产发现：统一列出可安装资产并显示类型
- 多目标安装：支持 `claude-code`、`codex`、`cursor`
- 安装校验：检查是否安装、版本是否最新、内容是否一致
- Import / Export：导入现有配置，导出到目标格式
- Shell Completion：生成 tab 补全脚本

## 安装

### 从源码安装

```bash
cargo install --locked --path crates/skillctrl-cli
```

### 本地开发构建

```bash
cargo build -p skillctrl
./target/debug/skillctrl --help
```

### 更新本地安装版本

```bash
cargo install --path crates/skillctrl-cli --force
```

## 快速示例

添加 source：

```bash
skillctrl source add team \
  --repo git@github.com:yourorg/skill-hub.git \
  --branch main \
  --ssh-key ~/.ssh/id_ed25519
```

列出资产：

```bash
skillctrl list --source team
```

安装资产：

```bash
skillctrl install review-pr \
  --source team \
  --target claude-code \
  --scope user
```

校验资产：

```bash
skillctrl verify review-pr \
  --source team \
  --target claude-code \
  --scope user
```

## 命令总览

```bash
skillctrl --help
skillctrl source --help
skillctrl list --help
skillctrl show --help
skillctrl install --help
skillctrl uninstall --help
skillctrl status --help
skillctrl verify --help
skillctrl update --help
skillctrl export --help
skillctrl import --help
skillctrl completion --help
```

## 文档

- 用户使用手册：[USER_GUIDE.md](./USER_GUIDE.md)

`USER_GUIDE.md` 包含：

- 每个命令的详细调用方式
- 参数说明和推荐写法
- `project` / `user` 作用域的使用建议
- `--json-resp` 的脚本化调用方式
- `completion`、`verify`、`import`、`export` 的完整示例

## 开发说明

如果你在修改 CLI 或适配器代码，建议至少执行：

```bash
cargo fmt
cargo test -p skillctrl
cargo build -p skillctrl
```
