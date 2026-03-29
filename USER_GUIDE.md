# skillctrl User Guide

`skillctrl` 是一个统一管理 AI 编码助手资产的命令行工具。它可以把同一份资产仓库中的 `skill`、`rule`、`mcp`、`resource`、`agent`、`command`、`hook` 等内容，安装到 Claude Code、Codex、Cursor 等不同目标环境中。

本文档面向最终用户，重点说明每个命令应该如何调用、参数分别代表什么、适合在什么场景下使用。

## 1. 能力概览

`skillctrl` 当前支持这些核心能力：

- 管理远程或本地 source 仓库
- 列出 source 中可用的资产
- 查看单个资产详情
- 将资产安装到 `project` 或 `user` 作用域
- 查询安装记录
- 校验资产是否已安装、版本是否最新、内容是否与 source 一致
- 更新 source 缓存
- 导出资产到目标格式
- 扫描并导入已有配置
- 生成 shell tab 补全脚本
- 所有命令支持 `--json-resp`，用于脚本化调用

## 2. 安装

### 从源码安装

```bash
cargo install --locked --path crates/skillctrl-cli
```

### 重新安装本地开发版本

如果你刚修改过当前仓库，推荐重新安装：

```bash
cargo install --path crates/skillctrl-cli --force
```

### 直接使用本地构建产物

```bash
cargo build -p skillctrl
./target/debug/skillctrl --help
```

### 构建并启动桌面应用

```bash
cd skillctrl-desktop
npm ci
npm run build
cd ..

cargo build -p skillctrl -p skillctrl-desktop
./target/debug/skillctrl-desktop
```

`skillctrl-desktop` 是跨平台桌面端，支持 macOS、Windows、Linux。当前桌面端基于 `Tauri + React + TypeScript + Vite`，通过调用 `skillctrl --json-resp` 来复用 CLI 的完整功能，因此开发环境下需要先构建前端，再构建两个二进制。

如果桌面端启动时提示找不到 `skillctrl`，按这个顺序检查：

- 确认 `skillctrl` 和 `skillctrl-desktop` 位于同一个目录
- 或者把 `skillctrl` 加到 `PATH`
- 或者设置 `SKILLCTRL_BINARY=/absolute/path/to/skillctrl`

## 3. 基本概念

### 3.1 Source

`source` 是一个 git 仓库，里面存放可供 `skillctrl` 使用的资产目录或 catalog。

常见形式：

- 本地目录：`/Users/you/my-skill-hub`
- SSH 仓库：`git@github.com:org/repo.git`
- HTTPS 仓库：`https://github.com/org/repo.git`

### 3.2 Asset

`skillctrl list` 展示的是“可安装资产”。每个资产通常对应一个 bundle，里面可能只包含一种组件，也可能包含多种组件。

你会在 `Type` 列看到这些类型：

- `skill`
- `rule`
- `command`
- `mcp`
- `hook`
- `resource`
- `agent`

### 3.3 Target

安装目标，也就是要把资产安装到哪个 AI 编码助手里。

当前常用值：

- `claude-code`
- `codex`
- `cursor`

### 3.4 Scope

安装作用域：

- `user`：安装到当前用户级目录
- `project`：安装到某个项目目录下

当 `scope=project` 时，通常需要额外传 `--project <项目路径>`。

### 3.5 Desktop App

`skillctrl-desktop` 是 `skillctrl` 的桌面图形界面。它覆盖当前 CLI 的主要能力，包括：

- Source 管理
- 资产列表与详情查看
- 安装、卸载、状态查询、内容校验
- Source 更新
- Import / Export
- Completion 脚本生成
- 原始 JSON、stdout、stderr 查看

桌面端更适合这些场景：

- 希望以表单方式填写参数，而不是手工拼接命令
- 需要更直观地查看资产类型、版本、安装状态和校验结果
- 想在一个界面里同时保留结构化结果和命令输出

当前桌面端技术栈：

- Tauri 原生壳
- React + TypeScript 页面层
- Vite 前端构建

## 4. 快速开始

### 4.1 添加一个 source

SSH 模式：

```bash
skillctrl source add team \
  --repo git@github.com:yourorg/skill-hub.git \
  --branch main \
  --ssh-key ~/.ssh/id_ed25519
```

HTTPS 模式：

```bash
skillctrl source add team \
  --repo https://github.com/yourorg/skill-hub.git \
  --branch main \
  --access-token <token>
```

本地目录模式：

```bash
skillctrl source add team-local \
  --repo /path/to/skill-hub
```

### 4.2 查看可用资产

```bash
skillctrl list
skillctrl list --source team
skillctrl list --source team --target codex
skillctrl list --search review
```

### 4.3 查看单个资产详情

```bash
skillctrl show review-pr --source team
```

### 4.4 安装资产

安装到用户级 Claude Code：

```bash
skillctrl install review-pr \
  --source team \
  --target claude-code \
  --scope user
```

安装到项目级 Codex：

```bash
skillctrl install review-pr \
  --source team \
  --target codex \
  --scope project \
  --project /path/to/repo
```

### 4.5 查看安装状态

```bash
skillctrl status --target claude-code --scope user
skillctrl status --target codex --scope project --project /path/to/repo
```

### 4.6 校验安装结果

```bash
skillctrl verify review-pr \
  --source team \
  --target claude-code \
  --scope user
```

它会告诉你：

- 这个资产有没有安装
- 本地安装版本是不是最新
- 本地文件内容是否与 source 中当前内容一致

### 4.7 启动桌面应用

开发环境下：

```bash
cd skillctrl-desktop
npm ci
npm run build
cd ..

cargo build -p skillctrl -p skillctrl-desktop
./target/debug/skillctrl-desktop
```

如果你是从 release 包启动，通常只需要保证两个二进制放在同一个目录下：

```bash
./skillctrl-desktop
```

桌面端会自动优先寻找同目录下的 `skillctrl`，找不到时再查找 `PATH` 或 `SKILLCTRL_BINARY`。

## 5. 全局参数

所有命令都支持以下全局参数。

### `--json-resp`

返回结构化 JSON，适合脚本、CI、自动化任务调用。

示例：

```bash
skillctrl --json-resp list
skillctrl source list --json-resp
skillctrl verify review-pr -S team -t codex -s user --json-resp
```

### `-v`, `--verbose`

启用更详细的日志输出。

```bash
skillctrl --verbose list
```

### `-q`, `--quiet`

关闭 spinner/progress bar。

```bash
skillctrl --quiet update
```

## 6. 命令详解

## 6.1 `source`

用于管理 source 仓库。

### `skillctrl source add`

添加一个新的 source。

调用格式：

```bash
skillctrl source add <NAME> --repo <REPO> [--branch <BRANCH>] [--ssh-key <PATH> | --access-token <TOKEN>]
```

参数说明：

- `<NAME>`：source 名称，后续通过 `--source` 或 `-S` 引用
- `--repo`：git 仓库地址，支持本地目录、SSH、HTTPS
- `--branch`：分支名，默认是 `main`
- `--ssh-key`：仅 SSH 仓库使用
- `--access-token`：仅 HTTPS 仓库使用

示例：

```bash
skillctrl source add hub --repo /Users/scl/myrepo/skillctrl-hub
skillctrl source add hub --repo git@github.com:org/repo.git --ssh-key ~/.ssh/id_ed25519
skillctrl source add hub --repo https://github.com/org/repo.git --access-token <token>
```

JSON 调用：

```bash
skillctrl source add hub --repo /path/to/repo --json-resp
```

### `skillctrl source list`

列出当前已配置的 source。

调用格式：

```bash
skillctrl source list
```

会显示：

- source 名称
- repo URL
- branch
- 认证方式摘要
- 最近一次同步 commit

JSON 调用：

```bash
skillctrl source list --json-resp
```

### `skillctrl source update`

更新指定 source 的本地缓存，也可以顺带更新认证参数。

调用格式：

```bash
skillctrl source update <NAME> [--ssh-key <PATH> | --access-token <TOKEN>]
```

示例：

```bash
skillctrl source update hub
skillctrl source update hub --ssh-key ~/.ssh/new_id_ed25519
skillctrl source update hub --access-token <new-token>
```

### `skillctrl source remove`

删除 source 配置，并清理对应缓存目录。

调用格式：

```bash
skillctrl source remove <NAME>
```

示例：

```bash
skillctrl source remove hub
```

## 6.2 `list`

列出所有可安装资产。

调用格式：

```bash
skillctrl list [--source <SOURCE>] [--target <TARGET>] [--search <TEXT>]
```

参数说明：

- `--source` / `-S`：只看某个 source
- `--target` / `-t`：只看支持某个 endpoint 的资产
- `--search` / `-s`：按关键字搜索

示例：

```bash
skillctrl list
skillctrl list --source hub
skillctrl list --target claude-code
skillctrl list --source hub --target codex --search review
```

默认文本输出是表格，列包括：

- `ID`
- `Type`
- `Source`
- `Version`
- `Targets`
- `Summary`

JSON 调用：

```bash
skillctrl list --json-resp
```

## 6.3 `show`

查看某个资产的详细信息。

调用格式：

```bash
skillctrl show <BUNDLE_ID> [--source <SOURCE>]
```

参数说明：

- `<BUNDLE_ID>`：资产 ID
- `--source` / `-S`：当同名资产在多个 source 中都存在时，建议显式指定

示例：

```bash
skillctrl show review-pr
skillctrl show review-pr --source hub
```

会显示：

- 资产 ID、名称、版本
- 来源 source
- 支持的 targets
- 描述
- 组件列表及组件路径

JSON 调用：

```bash
skillctrl show review-pr -S hub --json-resp
```

## 6.4 `install`

把某个资产安装到目标环境。

调用格式：

```bash
skillctrl install <BUNDLE_ID> --source <SOURCE> --target <TARGET> --scope <SCOPE> [--project <PROJECT>] [--dry-run]
```

参数说明：

- `<BUNDLE_ID>`：资产 ID
- `--source` / `-S`：从哪个 source 安装
- `--target` / `-t`：安装目标，常见为 `claude-code`、`codex`、`cursor`
- `--scope` / `-s`：`user` 或 `project`
- `--project` / `-p`：当 `scope=project` 时传项目目录
- `--dry-run`：只显示计划，不写文件

示例：

```bash
skillctrl install review-pr -S hub -t claude-code -s user
skillctrl install review-pr -S hub -t codex -s project -p /path/to/repo
skillctrl install review-pr -S hub -t cursor -s project -p /path/to/repo --dry-run
```

安装成功后会记录到本地 state DB，后续可通过 `status` 和 `verify` 查询。

JSON 调用：

```bash
skillctrl install review-pr -S hub -t claude-code -s user --json-resp
```

## 6.5 `status`

查看安装记录。

调用格式：

```bash
skillctrl status --target <TARGET> --scope <SCOPE> [--project <PROJECT>] [--bundle <BUNDLE_ID>]
```

参数说明：

- `--target` / `-t`：查询哪个 endpoint 的安装记录
- `--scope` / `-s`：`user` 或 `project`
- `--project` / `-p`：当查询项目级安装记录时传项目路径
- `--bundle` / `-b`：可选，只看某个资产

示例：

```bash
skillctrl status -t claude-code -s user
skillctrl status -t codex -s project -p /path/to/repo
skillctrl status -t codex -s project -p /path/to/repo -b review-pr
```

默认文本输出会显示：

- Bundle
- Version
- Source
- Installed At
- Files

JSON 调用：

```bash
skillctrl status -t claude-code -s user --json-resp
```

## 6.6 `verify`

校验某个资产在指定 target/scope 下的安装状态和内容状态。

调用格式：

```bash
skillctrl verify <BUNDLE_ID> --target <TARGET> --scope <SCOPE> [--source <SOURCE>] [--project <PROJECT>]
```

它会校验三类信息：

- 是否已安装
- 安装版本是否等于 source 中当前最新版本
- 本地内容是否与 source 中当前内容一致

示例：

```bash
skillctrl verify review-pr -S hub -t claude-code -s user
skillctrl verify review-pr -S hub -t codex -s project -p /path/to/repo
```

默认文本输出会显示总览，以及每个组件的校验结果表格。

JSON 调用：

```bash
skillctrl verify review-pr -S hub -t claude-code -s user --json-resp
```

典型 JSON 字段包括：

- `installed`
- `installed_version`
- `latest_version`
- `is_latest_version`
- `local_matches_source`
- `components`

## 6.7 `update`

更新 source 缓存。

调用格式：

```bash
skillctrl update [SOURCE]
```

说明：

- 不传参数时，更新全部 source
- 传入 `SOURCE` 时，只更新单个 source

示例：

```bash
skillctrl update
skillctrl update hub
```

JSON 调用：

```bash
skillctrl update --json-resp
skillctrl update hub --json-resp
```

## 6.8 `export`

将资产导出到指定目录。

调用格式：

```bash
skillctrl export <BUNDLE_ID> --source <SOURCE> --target <TARGET> --out <OUT> --format <FORMAT>
```

参数说明：

- `<BUNDLE_ID>`：资产 ID
- `--source` / `-S`：来源 source
- `--target` / `-t`：目标 endpoint/格式
- `--out` / `-o`：导出目录
- `--format` / `-f`：导出格式名

示例：

```bash
skillctrl export review-pr -S hub -t claude-code -o ./dist/review-pr -f native
```

JSON 调用：

```bash
skillctrl export review-pr -S hub -t claude-code -o ./dist/review-pr -f native --json-resp
```

## 6.9 `import`

用于把已有的本地配置扫描、规划并导入成 `skillctrl` 可管理的资产。

### `skillctrl import scan`

扫描现有目录，识别其中的资产。

调用格式：

```bash
skillctrl import scan --from <ENDPOINT> --path <PATH>
```

示例：

```bash
skillctrl import scan --from claude-code --path /path/to/project
```

### `skillctrl import plan`

基于扫描结果生成导入计划。

调用格式：

```bash
skillctrl import plan --from <ENDPOINT> --path <PATH> [--id <BUNDLE_ID>]
```

示例：

```bash
skillctrl import plan --from claude-code --path /path/to/project --id migrated-bundle
```

### `skillctrl import apply`

将导入计划写到输出目录。

调用格式：

```bash
skillctrl import apply --from <ENDPOINT> --path <PATH> --out <OUT>
```

示例：

```bash
skillctrl import apply --from claude-code --path /path/to/project --out ./imported-bundle
```

JSON 调用：

```bash
skillctrl import scan --from claude-code --path /path/to/project --json-resp
skillctrl import plan --from claude-code --path /path/to/project --id migrated-bundle --json-resp
skillctrl import apply --from claude-code --path /path/to/project --out ./imported-bundle --json-resp
```

## 6.10 `completion`

生成 shell tab 补全脚本。

调用格式：

```bash
skillctrl completion <SHELL>
```

支持的 shell：

- `bash`
- `elvish`
- `fish`
- `powershell`
- `zsh`

### zsh

```bash
mkdir -p ~/.zsh/completions
skillctrl completion zsh > ~/.zsh/completions/_skillctrl
```

然后在 `~/.zshrc` 中加入：

```bash
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit
```

### bash

```bash
mkdir -p ~/.local/share/bash-completion/completions
skillctrl completion bash > ~/.local/share/bash-completion/completions/skillctrl
```

### fish

```bash
mkdir -p ~/.config/fish/completions
skillctrl completion fish > ~/.config/fish/completions/skillctrl.fish
```

如果你希望把补全脚本作为 JSON 获取，也可以：

```bash
skillctrl completion zsh --json-resp
```

返回字段中会包含：

- `shell`
- `script`

## 6.11 `uninstall`

卸载资产。

调用格式：

```bash
skillctrl uninstall <BUNDLE_ID> --target <TARGET> --scope <SCOPE> [--project <PROJECT>] [--dry-run]
```

参数说明：

- `<BUNDLE_ID>`：资产 ID
- `--target` / `-t`：目标 endpoint
- `--scope` / `-s`：`user` 或 `project`
- `--project` / `-p`：项目级安装时传项目目录
- `--dry-run`：查看预期卸载动作

示例：

```bash
skillctrl uninstall review-pr -t claude-code -s user
skillctrl uninstall review-pr -t codex -s project -p /path/to/repo
skillctrl uninstall review-pr -t codex -s project -p /path/to/repo --dry-run
```

JSON 调用：

```bash
skillctrl uninstall review-pr -t claude-code -s user --json-resp
```

## 7. JSON 输出说明

所有命令都支持 `--json-resp`。推荐在自动化场景中统一使用它。

典型用途：

- shell 脚本
- CI/CD
- MCP/Agent 工具调用
- 外部程序二次封装

示例：

```bash
skillctrl source list --json-resp
skillctrl list --source hub --json-resp
skillctrl install review-pr -S hub -t claude-code -s user --json-resp
skillctrl verify review-pr -S hub -t claude-code -s user --json-resp
```

失败时也会返回结构化错误：

```json
{
  "success": false,
  "error": "..."
}
```

## 8. 常见使用场景

### 场景 1：维护团队统一资产仓库

```bash
skillctrl source add team --repo git@github.com:org/skill-hub.git --ssh-key ~/.ssh/id_ed25519
skillctrl update team
skillctrl list --source team
```

### 场景 2：给某个项目安装项目级规则

```bash
skillctrl install engineering-baseline \
  -S team \
  -t codex \
  -s project \
  -p /path/to/repo
```

### 场景 3：批量脚本检查资产是否漂移

```bash
skillctrl --json-resp verify review-pr -S team -t claude-code -s user
```

重点关注这些字段：

- `installed`
- `is_latest_version`
- `local_matches_source`

### 场景 4：切换 source 认证方式

SSH：

```bash
skillctrl source update hub --ssh-key ~/.ssh/id_ed25519
```

HTTPS：

```bash
skillctrl source update hub --access-token <token>
```

### 场景 5：使用桌面端管理 source 和资产

```bash
./skillctrl-desktop
```

推荐流程：

- 在 `Sources` 页面添加或更新 source
- 在 `Assets` 页面按 source、target、关键字筛选资产
- 在 `Install` 页面执行安装
- 在 `Status` 和 `Verify` 页面确认安装结果
- 在 `Console` 页面查看原始 JSON、stdout、stderr

## 9. 常见问题

### 9.1 `source add` 时 GitHub 超时

优先检查：

- 本机代理是否已启动
- `http_proxy` / `https_proxy` 是否配置正确
- SSH key 或 HTTPS token 是否有效

如果你本机系统 `git` 可以访问 GitHub，而 `skillctrl` 不行，先重新安装最新版本再重试。

### 9.2 `skillctrl-desktop` 启动时报找不到 `skillctrl`

桌面端需要调用 CLI 二进制。推荐把以下两个文件一起发布：

- `skillctrl`
- `skillctrl-desktop`

如果不能放在一起，可以设置：

```bash
export SKILLCTRL_BINARY=/absolute/path/to/skillctrl
```

然后再启动桌面端。

### 9.3 release 包里包含哪些文件

当前 release 归档会同时打包：

- `skillctrl`
- `skillctrl-desktop`
- `README.md`
- `USER_GUIDE.md`
- `LICENSE-Apache-2.0.txt`
- `BUILD_INFO.txt`

这样命令行和桌面端都可以直接从同一份发布物中使用。

### 9.4 `project` 作用域下必须传 `--project` 吗

是。凡是命令里需要明确项目目录时，`scope=project` 都应传：

```bash
--project /path/to/repo
```

### 9.5 `--json-resp` 放在前面还是后面

两种都可以，下面两种写法都推荐：

```bash
skillctrl --json-resp list
skillctrl list --json-resp
```

## 10. 参考命令清单

```bash
skillctrl source add <name> --repo <repo>
skillctrl source list
skillctrl source update <name>
skillctrl source remove <name>

skillctrl list
skillctrl show <bundle-id>
skillctrl install <bundle-id> -S <source> -t <target> -s <scope>
skillctrl uninstall <bundle-id> -t <target> -s <scope>
skillctrl status -t <target> -s <scope>
skillctrl verify <bundle-id> -t <target> -s <scope>
skillctrl update [source]
skillctrl export <bundle-id> -S <source> -t <target> -o <out> -f <format>

skillctrl import scan --from <endpoint> --path <path>
skillctrl import plan --from <endpoint> --path <path> --id <bundle-id>
skillctrl import apply --from <endpoint> --path <path> --out <out>

skillctrl completion zsh
skillctrl completion bash
skillctrl completion fish
```
