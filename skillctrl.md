# `skillctrl` 方案文档（Rust 跨平台版）

## 1. 项目目标

`skillctrl` 是一个**跨 Claude Code / Codex / Cursor 的统一内容仓库 + 单二进制工具**。

它要解决两件事：

1. **正向安装**

   * 只维护一份 skills / commands / MCP / rules / resources 等内容
   * 从 Git 仓库 + 分支读取内容清单
   * 让用户选择要安装的包
   * 安装到目标 endpoint（Claude Code、Codex、Cursor）

2. **反向导入**

   * 扫描用户当前已有的 Claude Code / Codex / Cursor 配置和内容
   * 把这些已有的 skills / commands / MCP / rules / resources 等，转换成 `skillctrl` 的统一模板
   * 生成可回收到统一仓库里的标准包

这个方向是可行的，因为三家现在都已经公开支持“可复用能力包”这类扩展面，但形态并不完全相同：Claude Code 已有 skills、plugins、MCP、hooks 和 marketplace；Codex 有 skills、plugins、AGENTS.md、rules、MCP 配置；Cursor 文档公开了 rules、skills 和 plugins，且 plugins 可以打包 rules、skills、agents、commands、MCP servers、hooks。([Claude API Docs][1])

---

## 2. 技术选型

### 2.1 语言选择

按你的约束，这个项目**不用 Go**，同时需要在 Windows / Linux / macOS 都能提供一致的 CLI 和分发体验。

我建议直接选 **Rust**。

原因不是“Rust 更潮”，而是它更适合这个项目的三个核心要求：

* 做成**单二进制**
* 做稳定的**跨平台文件系统操作**
* 做**多适配器 + 多导入器 + 多格式清单**这类偏系统工具的长期维护

### 2.2 建议的 Rust 技术栈

建议：

* CLI：`clap`
* TUI：第二期再加 `ratatui`
* 序列化：`serde`, `serde_yaml`, `toml`, `serde_json`
* 路径/目录遍历：`camino`, `walkdir`
* Git：

  * 第一阶段优先直接调用系统 `git`
  * 第二阶段再视需要引入 `gix`
* 模板渲染：`minijinja` 或 `handlebars`
* SQLite：`rusqlite`
* 校验：`jsonschema` 或自定义 schema
* 压缩包/导出：`tar`, `flate2`, `zip`

---

## 3. 产品边界

## 3.1 第一阶段必须支持

* 远程 Git 仓库 + 分支读取
* 列出 catalog
* 选择 package 安装到指定 endpoint
* 检查已安装状态
* 从现有 endpoint 内容反向导入为 `skillctrl` 模板
* 支持 project scope 和 user/global scope

## 3.2 第一阶段不做

* 账号系统
* SaaS 后台
* 官方 marketplace 提交自动化
* 图形界面桌面端
* 云端签名服务
* 多组织权限系统

---

## 4. 统一抽象模型

这部分是整个项目成败的关键。

不要按 `.claude/`、`.codex/`、`.cursor/` 维护三套源文件。
应该维护一套 **Canonical Model（统一内容模型）**，再由 adapter 落盘到不同 endpoint。

### 4.1 顶层对象：Bundle

`skillctrl` 的最小安装单元叫 **Bundle**。

一个 Bundle 可以包含多个组件：

* `skill`
* `rule`
* `command`
* `mcp_server`
* `hook`
* `resource`
* `agent`
* `plugin_meta`

之所以要这样拆，是因为这三家虽然都支持“扩展”，但天然的第一公民不一样：

* Claude Code：skills 和 plugins 很强，custom commands 已并入 skills；`.claude` 目录里还有 rules、subagents、settings，MCP 走 `.mcp.json` / `~/.claude.json`。([Claude API Docs][1])
* Codex：skills 是工作流内容，plugins 是安装/分发单元，`AGENTS.md` 是项目指导，MCP 放在 `config.toml`，rules 是实验性命令外放控制。([OpenAI 开发者][2])
* Cursor：公开文档片段显示 rules、skills、plugins 都存在，rules 支持 `.md` / `.mdc`，`.mdc` 可用 frontmatter 和 globs 控制作用范围；plugins 可打包 rules、skills、agents、commands、MCP servers、hooks。([Cursor][3])

### 4.2 统一 manifest

建议每个 bundle 用一个 `bundle.yaml`：

```yaml
apiVersion: skillctrl.dev/v1
kind: Bundle
id: review-pr
name: Review PR
version: 1.2.0
description: Review pull requests with architecture, tests, and security checks
authors:
  - name: team-ai
tags: [review, git, quality]
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
compat:
  claude-code:
    installMode: native-skill
  codex:
    installMode: native-skill
  cursor:
    installMode: rule-first
provenance:
  source:
    type: authored
```

### 4.3 统一 catalog

仓库根目录有一个 `catalog.yaml`：

```yaml
apiVersion: skillctrl.dev/v1
kind: Catalog
name: team-market
bundles:
  - id: review-pr
    version: 1.2.0
    path: bundles/review-pr
    summary: Pull request review workflow
  - id: api-design
    version: 0.8.0
    path: bundles/api-design
    summary: API design standards
```

---

## 5. 仓库结构

建议 `skillctrl` 仓库长这样：

```text
skillctrl/
  Cargo.toml
  crates/
    skillctrl-cli/
    skillctrl-core/
    skillctrl-catalog/
    skillctrl-manifest/
    skillctrl-git/
    skillctrl-state/
    skillctrl-adapter-claude/
    skillctrl-adapter-codex/
    skillctrl-adapter-cursor/
    skillctrl-importer-claude/
    skillctrl-importer-codex/
    skillctrl-importer-cursor/
    skillctrl-exporter/
  schemas/
    catalog.schema.json
    bundle.schema.json
  docs/
    architecture.md
    adapter-spec.md
    import-spec.md
  examples/
    market/
      catalog.yaml
      bundles/
        review-pr/
          bundle.yaml
          components/
            skills/
            rules/
            resources/
```

---

## 6. 核心命令设计

### 6.1 source 管理

```bash
skillctrl source add team \
  --repo git@github.com:yourorg/ai-market.git \
  --branch main

skillctrl source list
skillctrl source update team
```

### 6.2 catalog 浏览

```bash
skillctrl list --source team
skillctrl show review-pr --source team
skillctrl search review --source team
```

### 6.3 安装

```bash
skillctrl install review-pr \
  --source team \
  --target claude-code \
  --scope project \
  --project /path/to/repo
```

### 6.4 更新 / 卸载 / 状态

```bash
skillctrl update review-pr --target codex
skillctrl uninstall review-pr --target cursor
skillctrl status --project /path/to/repo
```

### 6.5 反向导入

```bash
skillctrl import scan --from claude-code --path /repo
skillctrl import plan --from codex --path /repo
skillctrl import apply --from cursor --path /repo --out ./market/bundles
```

### 6.6 导出 marketplace / native bundle

```bash
skillctrl export claude-marketplace --source ./market --out ./dist/claude-market
skillctrl export codex-plugin --bundle review-pr --out ./dist/review-pr-codex
skillctrl export cursor-plugin --bundle review-pr --out ./dist/review-pr-cursor
```

---

## 7. 正向安装设计

## 7.1 总体流程

安装流程统一为：

1. 读取 source 配置
2. `git clone/fetch + checkout` 到目标 branch
3. 解析 `catalog.yaml`
4. 解析对应 `bundle.yaml`
5. 选择目标 adapter
6. adapter 生成安装计划
7. 冲突检查
8. 落盘
9. 写安装记录
10. 输出结果

### 7.2 状态目录

本地建议：

```text
~/.config/skillctrl/
  sources.yaml
  cache/
    team/
      repo/
  state.db
  logs/
```

Windows 用等价的 AppData 路径。

### 7.3 安装记录

建议用 SQLite，至少记录：

* source
* bundle id / version
* target endpoint
* scope
* 安装文件列表
* 原始备份
* 安装时间
* 上次更新 commit / ref

---

## 8. 三个 endpoint 的适配策略

## 8.1 Claude Code adapter

Claude Code 的公开目录结构很清楚：项目和用户目录都可提供 `.claude` 内容；`.claude` 里会读 `CLAUDE.md`、`settings.json`、hooks、skills、commands、subagents、rules；MCP server 的位置是 `~/.claude.json` 和项目根的 `.mcp.json`；plugins 也支持 user / project / local scope。Claude Code 还支持 plugin marketplace，marketplace 可以来自 GitHub repo、任意 git URL、本地路径或远程 `marketplace.json` URL。([Claude][4])

### Claude 正向映射

* `skill` → `.claude/skills/<id>/SKILL.md`
* `command` → 优先转成 skill；因为官方已把 custom commands 合并进 skills，`.claude/commands/*.md` 仍兼容但不是未来主路径。([Claude API Docs][1])
* `rule` → `.claude/rules/<id>.md`
* `hook` → `.claude/settings.json` 的 hooks 节点
* `mcp_server` → `.mcp.json` 或 `~/.claude.json`
* `agent` → `.claude/agents/<id>/...`
* `plugin export` → `.claude-plugin/plugin.json` + plugin 目录；还可进一步导出 `.claude-plugin/marketplace.json` 做原生 Claude marketplace。([Claude][5])

### Claude 结论

对 `skillctrl` 来说，Claude Code 是**最成熟的原生出口**。
第一阶段建议把 Claude 作为“最完整 adapter”，同时支持：

* 直接安装到 `.claude`
* 导出成 Claude plugin
* 导出成 Claude marketplace

---

## 8.2 Codex adapter

Codex 的公开能力也比较完整：

* `AGENTS.md` 是项目/全局指导链
* `~/.codex/config.toml` 和 `.codex/config.toml` 是配置层
* skills 使用 `SKILL.md`，并支持 scripts / references / assets
* plugins 是可安装分发单元，可打包 skills、apps、MCP servers
* MCP servers 在 `config.toml` 里有成体系的配置键
* rules 是实验性能力，用于控制命令何时能在 sandbox 外运行。([OpenAI 开发者][6])

### Codex 正向映射

* `skill` → `skills/<id>/SKILL.md`
* `resource` → `references/` / `assets/`
* `rule` → 两类分开处理

  * 语义规则 / repo 约束 → `AGENTS.md` 片段
  * sandbox / command approval 规则 → `.codex/rules/*.rules`
* `mcp_server` → `.codex/config.toml` 的 `mcp_servers.<id>` 节点
* `plugin export` → `.codex-plugin/plugin.json` + `skills/` + `.mcp.json`

### Codex 结论

Codex 适配不能把所有“rule”都混成一类，因为它至少分成：

* 面向 agent 行为的 `AGENTS.md`
* 面向命令越权控制的 `.rules`

这意味着 `skillctrl` 的统一模型里，`rule` 至少要再细分成：

* `guidance_rule`
* `execution_rule`

---

## 8.3 Cursor adapter

Cursor 的公开文档片段显示：

* rules 支持 `.md` 和 `.mdc`
* `.mdc` 可以用 frontmatter 指定 description 和 globs
* skills 基于开放的 Agent Skills 标准
* plugins 可打包 rules、skills、agents、commands、MCP servers、hooks。([Cursor][3])

### Cursor 正向映射

第一阶段建议保守些：

* `rule` → `.cursor/rules/<id>.mdc`
* `skill` → 若 Cursor 当前目标面允许原生 skill，则安装为 skill；否则先降级成 `.mdc` rule + resource
* `mcp_server` → 仅做 exporter/importer 模型支持，落盘先走插件导出，不直接改用户未知结构
* `command` / `hook` → 第一阶段只在 plugin export 中支持
* `plugin export` → 输出 Cursor plugin bundle 目录结构

### Cursor 结论

Cursor 文档对“可以打包什么”说明很明确，但公开可见的本地落盘细节不像 Claude/Codex 那样完整。
所以第一阶段不要追求“全量本地直装”，而是做：

* **rules 直装**
* **skills/import/export 支持**
* **plugin 导出优先**

这是更稳的路线。([Cursor][7])

---

## 9. 反向导入设计

这是你新需求里最有价值的部分。

`skillctrl` 不只是 installer，而是 **installer + importer + normalizer**。

## 9.1 反向导入目标

把现有 endpoint 中的内容抽取成统一 bundle：

* 保留原始语义
* 标记来源
* 标记可能的有损转换
* 生成 `bundle.yaml`
* 生成规范化目录
* 生成 `compat-notes.md`

## 9.2 Claude importer

Claude 侧可导入的内容非常明确：

* `.claude/skills/**/SKILL.md`
* `.claude/commands/*.md`
* `.claude/rules/*.md`
* `.claude/agents/**`
* `.claude/settings.json` 中的 hooks
* `.mcp.json` / `~/.claude.json` 中的 MCP
* `.claude-plugin/plugin.json` 及其 plugin 目录
  Anthropic 官方文档甚至直接给了“把现有 `.claude/` 配置迁移为 plugin”的步骤：skills / commands / agents 可以直接复制，hooks 从 settings 迁到 `hooks/hooks.json`，然后用 `claude --plugin-dir` 测试。([Claude][8])

### Claude importer 规则

* `.claude/commands/deploy.md` → canonical `command`
* `.claude/skills/deploy/SKILL.md` → canonical `skill`
* `settings.json.hooks` → canonical `hook`
* `.mcp.json` → canonical `mcp_server`
* `rules/*.md` → canonical `guidance_rule`

## 9.3 Codex importer

Codex 侧建议导入：

* `skills/**/SKILL.md`
* `AGENTS.md` / `AGENTS.override.md`
* `.codex/config.toml`
* `.codex/rules/*.rules`
* `.codex-plugin/plugin.json`
* plugin 根目录里的 `skills/`、`.mcp.json`、`assets/`
  这是因为 Codex skills、plugins、AGENTS、MCP、rules 都是公开文档里的正式表面。([OpenAI 开发者][2])

### Codex importer 规则

* `AGENTS.md` → canonical `guidance_rule`
* `.codex/rules/*.rules` → canonical `execution_rule`
* `config.toml.mcp_servers.*` → canonical `mcp_server`
* `skills/*/SKILL.md` → canonical `skill`

## 9.4 Cursor importer

Cursor 第一阶段导入：

* `.cursor/rules/*.md`
* `.cursor/rules/*.mdc`
* 公开可识别的 skill 目录
* plugin manifest 与 bundle 目录
  由于公开文档片段主要确认了 rules / skills / plugins 三层能力，Importer 第一阶段应以这些公开表面为主。([Cursor][3])

### Cursor importer 规则

* `.md` / `.mdc` → canonical `guidance_rule`
* `.mdc` frontmatter → `triggers/globs/description`
* plugin 内 rules/skills → 分别映射为 canonical 组件

---

## 10. 反向导入的输出格式

导入后统一生成：

```text
bundles/
  imported-review-pr/
    bundle.yaml
    components/
      skills/
      rules/
      commands/
      hooks/
      mcp/
      resources/
    provenance.yaml
    compat-notes.md
```

### provenance.yaml 示例

```yaml
source:
  endpoint: claude-code
  scope: project
  path: /repo/.claude
  importedAt: 2026-03-28T12:00:00Z
  originalFiles:
    - .claude/commands/review.md
    - .claude/settings.json
loss:
  - command imported as canonical command; Claude-native command alias preserved
```

### compat-notes.md 示例

写明：

* 哪些字段可 1:1 保留
* 哪些字段在别的 endpoint 上只能降级
* 哪些字段无法导出

---

## 11. 有损转换策略

这部分一定要设计清楚。

### 11.1 必须接受“不能完全等价”

因为三家模型不同：

* Claude 的 commands 已并入 skills。([Claude API Docs][1])
* Codex 的 rules 主要是 sandbox 外命令控制，和 Claude/Cursor 的“提示规则”不是同一类。([OpenAI 开发者][9])
* Cursor 的 rules 是文档型规则，`.mdc` 还有 frontmatter/glob 触发。([Cursor][3])

### 11.2 skillctrl 的策略

统一模型中把“规则”拆开：

* `guidance_rule`：提示/流程/约束
* `execution_rule`：命令执行/权限/沙箱规则

统一模型中把“命令”单独建模：

* `command`
* 但目标 adapter 可以把它导出成 skill、rule、plugin component 或忽略

这样才能保证导入和导出都可解释。

---

## 12. 仓库与 marketplace 策略

你的“市场”不要直接绑定某一家官方 marketplace。
应该做成两层：

### 12.1 `skillctrl` 自己的 catalog 是事实源

仓库只维护：

* `catalog.yaml`
* `bundle.yaml`
* 统一内容目录

### 12.2 原生 marketplace 作为导出物

* Claude：可导出 `.claude-plugin/marketplace.json`，并且 Claude Code 原生支持从 GitHub repo、任意 git URL、本地路径或远程 URL 添加 marketplace。([Claude][10])
* Codex：可导出 plugin 目录结构；官方文档说明 plugins 是可安装分发单元，skills 是作者格式，plugin 是安装单位。([OpenAI 开发者][2])
* Cursor：导出 plugin bundle，后续再接它自己的更原生分发流。([Cursor][7])

结论就是：

**`skillctrl` 是市场内核；原生 marketplace/plugin 只是不同 endpoint 的发货格式。**

---

## 13. 可扩展架构设计

为了确保后续添加新 AI 软件 adapter 的可扩展性，需要采用**插件化架构**和**分层抽象**设计。

### 13.1 核心设计原则

1. **开闭原则**：对扩展开放，对修改关闭
2. **依赖倒置**：依赖抽象而非具体实现
3. **接口隔离**：每个接口职责单一
4. **插件发现**：支持动态 adapter 发现和加载
5. **版本兼容**：向后兼容的 manifest 格式

### 13.2 增强的 Rust 模块边界

```
skillctrl/
  crates/
    # 核心抽象层
    skillctrl-core/              # 公共类型和 trait 定义
    skillctrl-macros/            # 过程宏，减少样板代码

    # 领域层
    skillctrl-bundle/            # Bundle 处理逻辑
    skillctrl-catalog/           # Catalog 读取和解析
    skillctrl-component/         # 组件抽象和类型系统
    skillctrl-dependency/        # 依赖关系解析
    skillctrl-version/           # 版本管理和兼容性检查

    # 基础设施层
    skillctrl-git/               # Git 操作抽象
    skillctrl-fs/                # 文件系统操作抽象
    skillctrl-state/             # 状态管理
    skillctrl-cache/             # 缓存管理
    skillctrl-config/            # 配置管理
    skillctrl-registry/          # Adapter 注册表

    # 执行层
    skillctrl-pipeline/          # 安装/导入管道
    skillctrl-middleware/        # 中间件系统
    skillctrl-events/            # 事件系统
    skillctrl-validation/        # 校验框架

    # Adapter 层（可插拔）
    skillctrl-adapter-core/      # Adapter 基础 trait 和工具
    skillctrl-adapter-claude/
    skillctrl-adapter-codex/
    skillctrl-adapter-cursor/
    # 新 adapter 只需添加新 crate

    # Importer 层（可插拔）
    skillctrl-importer-core/     # Importer 基础 trait 和工具
    skillctrl-importer-claude/
    skillctrl-importer-codex/
    skillctrl-importer-cursor/
    # 新 importer 只需添加新 crate

    # 导出层（可插拔）
    skillctrl-exporter-core/     # Exporter 基础 trait 和工具
    skillctrl-exporter-claude/
    skillctrl-exporter-codex/
    skillctrl-exporter-cursor/
    # 新 exporter 只需添加新 crate

    # CLI 层
    skillctrl-cli/               # 命令行入口
```

### 13.3 核心类型系统

```rust
// skillctrl-core/src/types.rs

/// 核心抽象：可安装组件
pub trait Component: Any + Send + Sync {
    fn kind(&self) -> ComponentKind;
    fn id(&self) -> &str;
    fn validate(&self) -> Result<ValidationReport>;
    fn dependencies(&self) -> &[ComponentDependency];

    // 序列化支持
    fn as_serialize(&self) -> &dyn erased_serde::Serialize;
}

/// 组件种类枚举（可扩展）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentKind {
    Skill,
    Rule,
    Command,
    McpServer,
    Hook,
    Resource,
    Agent,
    PluginMeta,
    // 允许扩展
    Custom(&'static str),
}

/// 组件依赖关系
#[derive(Debug, Clone)]
pub struct ComponentDependency {
    pub component_id: String,
    pub kind: ComponentKind,
    pub version_constraint: Option<VersionReq>,
    pub required: bool,
}

/// 目标端点（可注册）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Endpoint {
    Known(KnownEndpoint),
    Custom(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnownEndpoint {
    ClaudeCode,
    Codex,
    Cursor,
}
```

### 13.4 Adapter 注册系统

```rust
// skillctrl-registry/src/lib.rs

/// 动态 adapter 注册表
pub struct AdapterRegistry {
    adapters: HashMap<Endpoint, Box<dyn Adapter>>,
    factories: HashMap<Endpoint, Box<dyn AdapterFactory>>,
}

impl AdapterRegistry {
    /// 注册新 adapter
    pub fn register(&mut self, endpoint: Endpoint, adapter: Box<dyn Adapter>) {
        self.adapters.insert(endpoint, adapter);
    }

    /// 注册 adapter 工厂（延迟初始化）
    pub fn register_factory<F>(&mut self, endpoint: Endpoint, factory: F)
    where
        F: AdapterFactory + 'static,
    {
        self.factories.insert(endpoint, Box::new(factory));
    }

    /// 从配置文件发现并注册 adapter
    pub fn discover_from_config(&mut self, config: &Config) -> Result<Vec<Endpoint>> {
        let mut discovered = Vec::new();

        for adapter_config in &config.adapters {
            let endpoint = adapter_config.endpoint.clone();
            let adapter: Box<dyn Adapter> = match adapter_config.loader.as_str() {
                "builtin" => self.load_builtin(&endpoint)?,
                "plugin" => self.load_plugin(&adapter_config.plugin_path)?,
                _ => return Err(Error::UnknownLoader),
            };
            self.adapters.insert(endpoint, adapter);
            discovered.push(endpoint);
        }

        Ok(discovered)
    }
}
```

### 13.5 增强的 Adapter Trait

```rust
// skillctrl-adapter-core/src/lib.rs

/// 核心 Adapter trait（拆分为多个小 trait）
pub trait Adapter: Send + Sync + 'static {
    fn endpoint(&self) -> Endpoint;
    fn version(&self) -> &str;

    // 能力查询
    fn capabilities(&self) -> AdapterCapabilities;

    // 生命周期钩子
    fn pre_install(&self, ctx: &InstallContext) -> Result<HookResult>;
    fn post_install(&self, result: &InstallResult) -> Result<HookResult>;
    fn pre_uninstall(&self, ctx: &UninstallContext) -> Result<HookResult>;
    fn post_uninstall(&self, result: &UninstallResult) -> Result<HookResult>;
}

/// 安装能力（独立 trait）
pub trait InstallAdapter: Adapter {
    fn plan_install(&self, bundle: &BundleManifest, ctx: &InstallContext)
        -> Result<InstallPlan>;

    fn apply_install(&self, plan: &InstallPlan) -> Result<InstallResult>;

    fn rollback_install(&self, plan: &InstallPlan) -> Result<RollbackResult>;
}

/// 导入能力（独立 trait）
pub trait ImportAdapter: Adapter {
    fn plan_import(&self, req: &ImportRequest) -> Result<ImportPlan>;

    fn apply_import(&self, plan: &ImportPlan) -> Result<ImportResult>;

    fn scan(&self, req: &ScanRequest) -> Result<DetectedArtifacts>;
}

/// 导出能力（独立 trait）
pub trait ExportAdapter: Adapter {
    fn plan_export(&self, req: &ExportRequest) -> Result<ExportPlan>;

    fn apply_export(&self, plan: &ExportPlan) -> Result<ExportResult>;
}

/// 状态查询能力（独立 trait）
pub trait StatusAdapter: Adapter {
    fn status(&self, req: &StatusRequest) -> Result<StatusReport>;

    fn validate_installation(&self, bundle_id: &str) -> Result<ValidationReport>;
}

/// Adapter 能力声明
#[derive(Debug, Clone, Copy)]
pub struct AdapterCapabilities {
    pub can_install: bool,
    pub can_import: bool,
    pub can_export: bool,
    pub can_query_status: bool,
    pub supported_scopes: Vec<Scope>,
    pub supported_kinds: Vec<ComponentKind>,
    pub max_manifest_version: semver::Version,
}

// 为简化使用，提供宏自动实现
#[macro_export]
macro_rules! impl_adapter {
    ($adapter:ty, $endpoint:expr) => {
        impl Adapter for $adapter {
            fn endpoint(&self) -> Endpoint { $endpoint }
            fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
            fn capabilities(&self) -> AdapterCapabilities { self.capabilities() }
            // ... 默认实现
        }
    };
}
```

### 13.6 管道和中间件系统

```rust
// skillctrl-pipeline/src/lib.rs

/// 安装管道
pub struct InstallPipeline {
    registry: Arc<AdapterRegistry>,
    middleware_stack: Vec<Box<dyn Middleware>>,
    event_emitter: EventEmitter,
}

impl InstallPipeline {
    pub fn new(registry: Arc<AdapterRegistry>) -> Self {
        Self {
            registry,
            middleware_stack: Vec::new(),
            event_emitter: EventEmitter::new(),
        }
    }

    /// 添加中间件
    pub fn use_middleware<M>(&mut self, middleware: M) -> &mut Self
    where
        M: Middleware + 'static,
    {
        self.middleware_stack.push(Box::new(middleware));
        self
    }

    /// 执行安装
    pub async fn execute(&self, req: InstallRequest) -> Result<InstallResult> {
        // 阶段 1: 前置中间件
        let ctx = self.run_before_middleware(&req).await?;

        // 发送事件
        self.event_emitter.emit(InstallEvent::Started {
            bundle_id: req.bundle_id.clone(),
        });

        // 阶段 2: 获取 adapter 并规划
        let adapter = self.registry.get(&req.target)?;
        let plan = adapter.plan_install(&req.bundle, &ctx)?;

        // 阶段 3: 验证和冲突检查
        self.validate_plan(&plan).await?;

        // 阶段 4: 执行安装
        let result = adapter.apply_install(&plan)?;

        // 阶段 5: 后置中间件
        let result = self.run_after_middleware(result).await?;

        // 发送完成事件
        self.event_emitter.emit(InstallEvent::Completed {
            bundle_id: req.bundle_id,
        });

        Ok(result)
    }
}

/// 中间件 trait
#[async_trait]
pub trait Middleware: Send + Sync + 'static {
    async fn before_install(
        &self,
        req: &InstallRequest,
        ctx: &mut InstallContext,
    ) -> Result<()>;

    async fn after_install(
        &self,
        result: &InstallResult,
    ) -> Result<InstallResult>;
}

/// 内置中间件示例
pub struct ConflictDetectionMiddleware;
pub struct BackupMiddleware;
pub struct ValidationMiddleware;
pub struct LoggingMiddleware;
```

### 13.7 事件系统

```rust
// skillctrl-events/src/lib.rs

/// 事件发射器
pub struct EventEmitter {
    listeners: Arc<RwLock<Vec<Box<dyn EventHandler>>>>,
}

impl EventEmitter {
    pub fn subscribe<H>(&self, handler: H)
    where
        H: EventHandler + 'static,
    {
        self.listeners.write().push(Box::new(handler));
    }

    pub fn emit(&self, event: Event) {
        for handler in self.listeners.read().iter() {
            handler.handle(event.clone());
        }
    }
}

/// 事件类型
#[derive(Clone, Debug)]
pub enum Event {
    Install(InstallEvent),
    Import(ImportEvent),
    Export(ExportEvent),
}

#[derive(Clone, Debug)]
pub enum InstallEvent {
    Started { bundle_id: String },
    Progress { bundle_id: String, progress: f64 },
    Completed { bundle_id: String },
    Failed { bundle_id: String, error: String },
}

/// 事件处理器
pub trait EventHandler: Send + Sync {
    fn handle(&self, event: Event);
}

/// 内置事件处理器
pub struct LoggingEventHandler;
pub struct MetricsEventHandler;
pub struct WebhookEventHandler {
    webhook_url: String,
}
```

### 13.8 配置驱动的 Adapter 发现

```yaml
# ~/.config/skillctrl/adapters.yaml
apiVersion: skillctrl.dev/v1

# 内置 adapter
builtin:
  - endpoint: claude-code
    enabled: true
  - endpoint: codex
    enabled: true
  - endpoint: cursor
    enabled: false

# 插件 adapter
plugins:
  - name: windsurf
    endpoint: windsurf
    library: "/usr/local/lib/skillctrl-windsurf.so"
    config:
      # adapter 特定配置
      custom_rules_dir: "~/.windsurf/rules"

  - name: copilot
    endpoint: copilot
    library: "/usr/local/lib/skillctrl-copilot.so"
    enabled: false

# 远程 adapter（WASM 支持）
remote:
  - name: custom-ai
    endpoint: custom-ai
    url: "https://example.com/adapters/custom-ai.wasm"
    checksum: "sha256:..."
```

### 13.9 版本兼容性管理

```rust
// skillctrl-version/src/lib.rs

/// Manifest 版本策略
pub struct VersionPolicy {
    pub min_supported: semver::Version,
    pub max_supported: semver::Version,
    pub deprecated: Vec<semver::Version>,
}

/// 兼容性检查器
pub trait CompatibilityChecker {
    fn check_bundle_version(&self, bundle: &BundleManifest) -> Result<CompatibilityReport>;

    fn check_adapter_compatibility(
        &self,
        adapter: &dyn Adapter,
        bundle: &BundleManifest,
    ) -> Result<CompatibilityReport>;

    /// 尝试迁移旧版本
    fn migrate_bundle(&self, bundle: &BundleManifest, target: semver::Version)
        -> Result<BundleManifest>;
}

/// 自动迁移系统
pub struct Migrator {
    migrations: Vec<Migration>,
}

pub struct Migration {
    pub from_version: semver::Version,
    pub to_version: semver::Version,
    pub migrate_fn: fn(BundleManifest) -> Result<BundleManifest>,
}
```

### 13.10 组件依赖解析

```rust
// skillctrl-dependency/src/lib.rs

/// 依赖关系解析器
pub struct DependencyResolver {
    component_registry: Arc<ComponentRegistry>,
}

impl DependencyResolver {
    /// 解析安装顺序
    pub fn resolve_install_order(
        &self,
        components: &[Component],
    ) -> Result<Vec<Component>> {
        let graph = self.build_dependency_graph(components)?;
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();

        for component in components {
            self.visit(component, &graph, &mut sorted, &mut visited)?;
        }

        // 检测循环依赖
        if self.has_cycles(&graph) {
            return Err(Error::CircularDependency);
        }

        Ok(sorted)
    }

    /// 检测冲突
    pub fn detect_conflicts(
        &self,
        existing: &[InstalledComponent],
        new: &[Component],
    ) -> Result<Vec<Conflict>> {
        // 检测 ID 冲突、路径冲突、能力冲突等
    }
}
```

### 13.11 添加新 Adapter 的流程

有了以上架构，添加新 adapter 只需：

**步骤 1**：创建新的 crate
```bash
cargo new --lib skillctrl-adapter-windsurf
```

**步骤 2**：实现必要的 trait
```rust
use skillctrl_adapter_core::*;

#[derive(Debug)]
pub struct WindsurfAdapter {
    config: WindsurfConfig,
}

impl_adapter!(WindsurfAdapter, Endpoint::Custom("windsurf".into()));

impl InstallAdapter for WindsurfAdapter {
    fn plan_install(&self, bundle: &BundleManifest, ctx: &InstallContext)
        -> Result<InstallPlan>
    {
        // Windsurf 特定逻辑
    }

    fn apply_install(&self, plan: &InstallPlan) -> Result<InstallResult> {
        // 安装到 .windsurf/
    }
}

impl ImportAdapter for WindsurfAdapter {
    // 实现导入逻辑
}
```

**步骤 3**：注册 adapter
```yaml
# adapters.yaml
plugins:
  - name: windsurf
    endpoint: windsurf
    library: "target/libskillctrl_adapter_windsurf.so"
```

**步骤 4**：无需修改核心代码，直接使用
```bash
skillctrl install my-bundle --target windsurf
```

---

## 14. 核心接口设计（已整合进第 13 节）

核心接口设计已完全整合进第 13 节的可扩展架构中。以下是关键接口的快速参考：

### 14.1 核心接口层次

```
              ┌─────────────────────────────────────┐
              │          Adapter (base)              │
              │  - endpoint(), version(), caps()      │
              │  - lifecycle hooks                   │
              └─────────────────┬───────────────────┘
                  ┌─────────────┼─────────────┐
                  │             │             │
         ┌────────┴────┐  ┌────┴────┐  ┌────┴────┐
         │ InstallAdap │  │ImportAdap│  │ExportAdap│
         └────────────┘  └─────────┘  └─────────┘
         ┌────────────┐  ┌────────┐  ┌───────────┐
         │StatusAdapt │  │         │  │           │
         └────────────┘  └────────┘  └───────────┘
```

### 14.2 扩展点总结

| 扩展点 | 接口/机制 | 用途 |
|--------|-----------|------|
| 新 Endpoint | `Adapter` trait 实现 | 支持新的 AI 软件 |
| 新组件类型 | `ComponentKind::Custom` | 支持新的扩展类型 |
| 自定义逻辑 | `Middleware` trait | 在安装流程中插入逻辑 |
| 事件处理 | `EventHandler` trait | 响应安装/导入事件 |
| 版本迁移 | `Migration` 注册 | 支持旧版本 bundle |
| 配置扩展 | `adapter.config` YAML | adapter 特定配置 |
| 远程加载 | WASM plugin 接口 | 动态加载 adapter |

---

## 15. 冲突解决策略

### 15.1 内建冲突策略

```rust
/// 冲突解决策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStrategy {
    /// 跳过冲突文件
    Skip,

    /// 覆盖冲突文件
    Overwrite,

    /// 备份后写入
    BackupThenWrite,

    /// 询问用户
    Prompt,

    /// 重命名（添加后缀）
    Rename,

    /// 使用自定义处理器
    Custom(String),
}

/// 冲突类型
#[derive(Debug, Clone)]
pub enum Conflict {
    /// 文件路径冲突
    FilePath {
        existing: String,
        new: String,
        existing_bundle: String,
    },

    /// 组件 ID 冲突
    ComponentId {
        id: String,
        existing_kind: ComponentKind,
        new_kind: ComponentKind,
    },

    /// 能力冲突（如两个 rule 冲突）
    Capability {
        capability: String,
        existing_bundle: String,
    },

    /// 依赖冲突
    Dependency {
        component: String,
        required: String,
        existing: String,
    },
}

/// 可扩展的冲突处理器
pub trait ConflictResolver: Send + Sync {
    fn resolve(&self, conflict: &Conflict) -> Result<ConflictResolution>;

    fn batch_resolve(&self, conflicts: &[Conflict]) -> Result<Vec<ConflictResolution>> {
        conflicts.iter().map(|c| self.resolve(c)).collect()
    }
}

/// 内置冲突处理器
pub struct DefaultConflictResolver {
    strategy: ConflictStrategy,
}

impl ConflictResolver for DefaultConflictResolver {
    fn resolve(&self, conflict: &Conflict) -> Result<ConflictResolution> {
        match self.strategy {
            ConflictStrategy::Skip => Ok(ConflictResolution::Skip),
            ConflictStrategy::Overwrite => Ok(ConflictResolution::Overwrite),
            ConflictStrategy::BackupThenWrite => Ok(ConflictResolution::BackupAndWrite),
            ConflictStrategy::Prompt => {
                // 调用 CLI 提示用户
                Self::prompt_user(conflict)
            }
            _ => Ok(ConflictResolution::Skip),
        }
    }
}
```

### 15.2 配置驱动的冲突策略

```yaml
# ~/.config/skillctrl/conflict.yaml
strategies:
  default: backup-then-write

  # 按组件类型定制
  by_kind:
    skill: prompt
    rule: skip
    command: overwrite

  # 按 endpoint 定制
  by_endpoint:
    claude-code:
      rule: backup-then-write
    codex:
      rule: skip

  # 按文件模式定制
  by_pattern:
    - pattern: "**/settings.json"
      strategy: merge  # 合并而非覆盖
    - pattern: "**/*.md"
      strategy: backup-then-write
```

### 15.3 导入时的冲突判定

导入时需要检测：

1. **同名 bundle 已存在**
   ```rust
   if state.bundle_exists(&imported_bundle.id) {
       // 提示用户：覆盖 / 跳过 / 重命名
   }
   ```

2. **组件类型冲突**
   ```rust
   if let Some(existing) = state.get_component(&component.id) {
       if existing.kind != component.kind {
           return Err(ConflictError::ComponentKindMismatch {
               id: component.id,
               was: existing.kind,
               now: component.kind,
           });
       }
   }
   ```

3. **文件占用冲突**
   ```rust
   if let Some(owner) = state.who_owns(&file_path) {
       conflicts.push(Conflict::FilePath {
           existing: file_path.clone(),
           new: file_path,
           existing_bundle: owner,
       });
   }
   ```

### 15.4 智能合并

对于某些可合并的文件（如 `settings.json`），支持智能合并：

```rust
pub trait Merger: Send + Sync {
    fn can_merge(&self, file_type: &str) -> bool;

    fn merge(&self, existing: &Value, new: &Value) -> Result<Value>;
}

/// JSON 合并器（支持深度合并）
pub struct JsonMerger;

impl Merger for JsonMerger {
    fn can_merge(&self, file_type: &str) -> bool {
        file_type == "json"
    }

    fn merge(&self, existing: &Value, new: &Value) -> Result<Value> {
        match (existing, new) {
            (Value::Object(a), Value::Object(b)) => {
                let mut merged = a.clone();
                for (key, value) in b {
                    if merged.contains_key(key) {
                        // 递归合并
                        merged.insert(key.clone(), self.merge(&merged[key], value)?);
                    } else {
                        merged.insert(key.clone(), value.clone());
                    }
                }
                Ok(Value::Object(merged))
            }
            (_, new) => Ok(new.clone()),
        }
    }
}
```

---

## 16. 发布策略

### 16.1 多平台二进制发布

使用 GitHub Actions matrix 编译：

```yaml
# .github/workflows/release.yml
strategy:
  matrix:
    include:
      - target: x86_64-unknown-linux-musl
        os: ubuntu-latest
        artifact: skillctrl-linux-x64.tar.gz
      - target: x86_64-pc-windows-msvc
        os: windows-latest
        artifact: skillctrl-windows-x64.zip
      - target: x86_64-apple-darwin
        os: macos-latest
        artifact: skillctrl-macos-x64.tar.gz
      - target: aarch64-apple-darwin
        os: macos-latest
        artifact: skillctrl-macos-arm64.tar.gz
```

### 16.2 插件 SDK 发布

为了支持第三方开发 adapter，同时发布 SDK：

```toml
# skillctrl-sdk/Cargo.toml
[package]
name = "skillctrl-sdk"
version = "0.1.0"

[dependencies]
skillctrl-core = { path = "../skillctrl-core" }
skillctrl-adapter-core = { path = "../skillctrl-adapter-core" }
skillctrl-macros = { path = "../skillctrl-macros" }
```

第三方开发者只需：

```toml
[dependencies]
skillctrl-sdk = "0.1"
```

### 16.3 WASM 插件支持

支持 WebAssembly 作为远程插件格式：

```rust
// skillctrl-wasm/src/lib.rs

use wasmtime::*;

pub struct WasmAdapterLoader {
    engine: Engine,
    linker: Linker<AdapterState>,
}

impl WasmAdapterLoader {
    pub fn load_from_url(&self, url: &str) -> Result<Box<dyn Adapter>> {
        // 下载 WASM 模块
        let wasm_bytes = self.download(url)?;

        // 编译并实例化
        let module = Module::from_binary(&self.engine, &wasm_bytes)?;
        let mut store = Store::new(&self.engine, AdapterState::new());

        // 暴露 host 函数给 WASM
        self.linker.define(
            "skillctrl",
            "log",
            Func::wrap(&mut store, |msg: &str| {
                log::info!("WASM adapter: {}", msg);
            }),
        )?;

        let instance = self.linker.instantiate(&mut store, &module)?;

        // 导出 Adapter trait 实现
        Ok(Box::new(WasmAdapter::new(instance)))
    }
}
```

### 16.4 版本管理策略

* **Semantic Versioning**：严格遵循 semver
* **API Stability**：major 版本间保证向后兼容
* **Deprecation Policy**：至少提前两个 minor 版本声明弃用
* **Migration Guide**：提供自动迁移工具

---

## 17. 实施路线图

基于可扩展架构，按阶段实施：

### Phase 1：核心基础设施（2-3 周）

**目标**：搭建可扩展的底层框架

```
□ 核心 trait 定义
  ├─ Component trait
  ├─ Adapter trait（分层）
  ├─ Importer trait
  ├─ Exporter trait
  └─ EventHandler trait

□ 基础设施 crates
  ├─ skillctrl-core（公共类型）
  ├─ skillctrl-config（配置管理）
  ├─ skillctrl-registry（adapter 注册）
  ├─ skillctrl-state（状态管理）
  ├─ skillctrl-cache（缓存）
  └─ skillctrl-macros（减少样板代码）

□ 管道和中间件
  ├─ skillctrl-pipeline（安装/导入管道）
  ├─ skillctrl-middleware（中间件系统）
  └─ skillctrl-events（事件系统）

□ 依赖和版本
  ├─ skillctrl-dependency（依赖解析）
  └─ skillctrl-version（版本管理）

□ CLI 骨架
  └─ skillctrl-cli（基础命令结构）
```

**里程碑**：可以注册 mock adapter 并执行 dry-run 安装

### Phase 2：Claude Code 完整支持（3-4 周）

**目标**：实现第一个生产级 adapter

```
□ Claude adapter
  ├─ skillctrl-adapter-claude
  │   ├─ InstallAdapter 实现
  │   ├─ ImportAdapter 实现
  │   ├─ ExportAdapter 实现
  │   └─ StatusAdapter 实现
  └─ Claude 特定逻辑
      ├─ .claude/skills/ 映射
      ├─ .claude/commands/ 映射
      ├─ .claude/rules/ 映射
      ├─ .claude/settings.json hooks
      ├─ .claude/agents/ 映射
      ├─ .mcp.json 处理
      └─ .claude-plugin/ 导出

□ Catalog 和 Bundle
  ├─ skillctrl-catalog（catalog.yaml 解析）
  ├─ skillctrl-bundle（bundle.yaml 解析）
  └─ 示例 catalog 和 bundle

□ Git 集成
  ├─ skillctrl-git（git 操作）
  └─ source add/list/update 命令

□ 完整安装流程
  ├─ list/show/install 命令
  ├─ update/uninstall 命令
  ├─ status 命令
  └─ 冲突检测和解决
```

**里程碑**：可以完整安装 bundle 到 Claude Code

### Phase 3：Codex 和 Cursor 支持（4-5 周）

**目标**：验证架构的可扩展性

```
□ Codex adapter
  ├─ skillctrl-adapter-codex
  ├─ skillctrl-importer-codex
  └─ skillctrl-exporter-codex

□ Cursor adapter
  ├─ skillctrl-adapter-cursor
  ├─ skillctrl-importer-cursor
  └─ skillctrl-exporter-cursor

□ 测试三个 endpoint 互操作
  ├─ Claude → Codex 导入
  ├─ Codex → Cursor 导入
  └─ 统一 bundle → 多端安装
```

**里程碑**：三个 endpoint 可以互操作

### Phase 4：插件系统和扩展（3-4 周）

**目标**：支持第三方扩展

```
□ 插件加载
  ├─ 动态库加载（.so/.dylib/.dll）
  ├─ 配置驱动的 adapter 发现
  └─ 适配器沙箱

□ WASM 支持
  ├─ skillctrl-wasm crate
  ├─ 远程 adapter 加载
  └─ WASM ABI 定义

□ SDK 发布
  ├─ skillctrl-sdk crate
  ├─ 示例 adapter 模板
  └─ 开发者文档
```

**里程碑**：可以加载第三方 adapter

### Phase 5：企业功能（按需）

**目标**：支持团队协作

```
□ 签名和验证
  ├─ Bundle 签名
  ├─ 来源验证
  └─ 信任链管理

□ 权限系统
  ├─ 多组织支持
  ├─ RBAC
  └─ 审计日志

□ 高级导出
  ├─ 原生 marketplace 导出
  ├─ 批量操作
  └─ CI/CD 集成
```

### Phase 6：生态建设（持续）

```
□ 官方 marketplace
□ 第三方 adapter 生态
□ 社区 bundle 仓库
□ 集成测试套件
```

---

## 18. 首个实现提示词（更新版）

基于可扩展架构的首个实施提示词：

---

请帮我从零实现一个 Rust 项目，名字叫 `skillctrl`。

**项目目标**：
做一个跨 Claude Code / Codex / Cursor 的统一 skills 市场工具。采用**插件化架构**，确保后续可以轻松添加其他 AI 软件的 adapter。

**核心设计原则**：
1. 所有 adapter 通过 trait 定义，可动态注册
2. 核心逻辑不依赖任何具体 adapter 实现
3. 支持配置驱动的 adapter 发现
4. 中间件系统支持在安装流程中插入自定义逻辑

**第一阶段（Phase 1）必须实现**：

### 1. Rust workspace 结构
```
skillctrl/
  Cargo.toml
  crates/
    skillctrl-core/          # 公共类型和 trait
    skillctrl-macros/        # 过程宏
    skillctrl-config/        # 配置管理
    skillctrl-registry/      # Adapter 注册表
    skillctrl-state/         # 状态管理
    skillctrl-cache/         # 缓存管理
    skillctrl-pipeline/      # 安装/导入管道
    skillctrl-middleware/    # 中间件系统
    skillctrl-events/        # 事件系统
    skillctrl-dependency/    # 依赖解析
    skillctrl-version/       # 版本管理
    skillctrl-adapter-core/  # Adapter 基础 trait
    skillctrl-cli/           # CLI 入口
```

### 2. 核心 trait 定义
```rust
// skillctrl-core/src/lib.rs

pub trait Component: Any + Send + Sync {
    fn kind(&self) -> ComponentKind;
    fn id(&self) -> &str;
    fn validate(&self) -> Result<ValidationReport>;
    fn dependencies(&self) -> &[ComponentDependency];
}

pub trait Adapter: Send + Sync + 'static {
    fn endpoint(&self) -> Endpoint;
    fn version(&self) -> &str;
    fn capabilities(&self) -> AdapterCapabilities;
    fn pre_install(&self, ctx: &InstallContext) -> Result<HookResult>;
    fn post_install(&self, result: &InstallResult) -> Result<HookResult>;
}

pub trait InstallAdapter: Adapter {
    fn plan_install(&self, bundle: &BundleManifest, ctx: &InstallContext)
        -> Result<InstallPlan>;
    fn apply_install(&self, plan: &InstallPlan) -> Result<InstallResult>;
    fn rollback_install(&self, plan: &InstallPlan) -> Result<RollbackResult>;
}

pub trait ImportAdapter: Adapter {
    fn scan(&self, req: &ScanRequest) -> Result<DetectedArtifacts>;
    fn plan_import(&self, req: &ImportRequest) -> Result<ImportPlan>;
    fn apply_import(&self, plan: &ImportPlan) -> Result<ImportResult>;
}

pub trait Middleware: Send + Sync + 'static {
    async fn before_install(&self, req: &InstallRequest, ctx: &mut InstallContext)
        -> Result<()>;
    async fn after_install(&self, result: &InstallResult) -> Result<InstallResult>;
}

pub trait EventHandler: Send + Sync {
    fn handle(&self, event: Event);
}
```

### 3. Adapter 注册系统
```rust
// skillctrl-registry/src/lib.rs

pub struct AdapterRegistry {
    adapters: HashMap<Endpoint, Box<dyn Adapter>>,
    factories: HashMap<Endpoint, Box<dyn AdapterFactory>>,
}

impl AdapterRegistry {
    pub fn register(&mut self, endpoint: Endpoint, adapter: Box<dyn Adapter>);
    pub fn get(&self, endpoint: &Endpoint) -> Result<&dyn Adapter>;
    pub fn discover_from_config(&mut self, config: &Config) -> Result<Vec<Endpoint>>;
}
```

### 4. 安装管道
```rust
// skillctrl-pipeline/src/lib.rs

pub struct InstallPipeline {
    registry: Arc<AdapterRegistry>,
    middleware_stack: Vec<Box<dyn Middleware>>,
    event_emitter: EventEmitter,
}

impl InstallPipeline {
    pub fn new(registry: Arc<AdapterRegistry>) -> Self;
    pub fn use_middleware<M>(&mut self, middleware: M) -> &mut Self;
    pub async fn execute(&self, req: InstallRequest) -> Result<InstallResult>;
}
```

### 5. CLI 命令（骨架）
```bash
skillctl source add --repo <git-url> --branch <branch>
skillctl source list
skillctl list --source <name>
skillctl show <bundle-id> --source <name>
skillctl install <bundle-id> --source <name> --target <endpoint> --scope <project|user>
skillctl status
```

### 6. 配置文件格式
```yaml
# ~/.config/skillctrl/config.yaml
sources:
  - name: team
    repo: git@github.com:yourorg/market.git
    branch: main

adapters:
  - endpoint: claude-code
    enabled: true
    loader: builtin
```

### 7. 示例 Mock Adapter
创建一个 mock adapter 用于测试整个框架

### 8. 测试
- 单元测试：每个 trait 方法
- 集成测试：完整安装流程
- dry-run 支持

### 9. 文档
- README.md
- ARCHITECTURE.md（说明架构）
- CONTRIBUTING.md（说明如何添加新 adapter）

**请按下面顺序输出**：
1. 项目目录结构
2. 核心数据结构（ComponentKind, Endpoint, AdapterCapabilities 等）
3. 完整的 trait 定义
4. AdapterRegistry 实现
5. InstallPipeline 实现
6. CLI 骨架（使用 clap）
7. Mock Adapter 示例
8. 然后开始生成代码

**重要**：
- 使用 `erased-serde` 实现类型擦除的序列化
- 使用 `anyhow` 进行错误处理
- 使用 `tracing` 进行日志
- 使用 `tokio` 作为异步运行时
- 所有 trait 设计要考虑未来的扩展性

---

## 19. 最终建议

### 19.1 架构核心要点

**`skillctrl` 应该设计成**：

| 维度 | 选择 | 理由 |
|------|------|------|
| **语言** | Rust | 单二进制、跨平台、类型安全 |
| **架构** | 插件化 | Adapter 可动态注册，易于扩展 |
| **抽象** | 统一 canonical model | 一套内容模型，多端适配 |
| **分发** | 自有 catalog + 原生导出 | 独立于任何单一平台 |
| **能力** | 安装 + 导入 + 导出 | 双向互操作 |

### 19.2 可扩展性保证

新架构通过以下方式保证可扩展性：

1. **Trait 分离**：`Adapter`、`InstallAdapter`、`ImportAdapter`、`ExportAdapter` 独立
2. **动态注册**：`AdapterRegistry` 支持运行时注册新 adapter
3. **中间件系统**：可在安装流程中插入自定义逻辑
4. **事件系统**：解耦核心逻辑和副作用
5. **配置驱动**：通过 YAML 发现和配置 adapter
6. **WASM 支持**：支持远程加载第三方 adapter
7. **SDK 发布**：第三方可开发自己的 adapter

### 19.3 添加新 Adapter 的成本

有了新架构，添加新 adapter 只需：

```
1. 创建 crate (skillctrl-adapter-xxx)
2. 实现 trait
3. 配置文件中注册
4. 无需修改核心代码
```

预计工作量：**1-2 周**（如果 endpoint 结构清晰）

### 19.4 与旧方案对比

| 方面 | 旧方案 | 新方案 |
|------|--------|--------|
| Adapter 添加 | 修改核心代码 | 配置驱动注册 |
| 扩展点 | 固定接口 | Trait 组合 + 中间件 |
| 依赖管理 | 无 | 完整的依赖解析 |
| 冲突处理 | 简单策略 | 可扩展处理器 |
| 版本管理 | 无 | 完整的迁移系统 |
| 第三方扩展 | 不支持 | SDK + WASM |

### 19.5 下一步行动

建议按以下顺序进行：

1. **立即可做**：
   - 使用更新后的"首个实现提示词"（第 18 节）开始 Phase 1
   - 搭建核心 trait 和注册系统
   - 实现 mock adapter 验证架构

2. **Phase 1 完成后**：
   - 实现 Claude Code adapter
   - 验证端到端安装流程
   - 编写集成测试

3. **验证架构后再添加**：
   - Codex adapter
   - Cursor adapter
   - 原生 marketplace 导出

### 19.6 架构验证清单

在开始实现完整功能前，验证：

- [ ] 可以动态注册新 adapter
- [ ] adapter 之间完全隔离
- [ ] 中间件可以拦截和修改安装流程
- [ ] 事件系统可以解耦副作用
- [ ] 配置文件可以控制 adapter 行为
- [ ] 错误处理清晰且可恢复
- [ ] 测试可以 mock 任何 adapter

### 19.7 总结

新架构的核心优势：

**对开发者**：
- 清晰的 trait 定义
- 低成本添加新 adapter
- 完整的工具链支持

**对用户**：
- 统一的命令和体验
- 可靠的冲突处理
- 灵活的配置选项

**对生态**：
- SDK 支持第三方扩展
- WASM 支持远程 adapter
- 向后兼容的版本管理

[1]: https://docs.anthropic.com/en/docs/claude-code/slash-commands "Extend Claude with skills - Claude Code Docs"
[2]: https://developers.openai.com/codex/skills/ "Agent Skills – Codex | OpenAI Developers"
[3]: https://cursor.com/docs/rules?utm_source=chatgpt.com "Rules | Cursor Docs"
[4]: https://code.claude.com/docs/en/claude-directory "Explore the .claude directory - Claude Code Docs"
[5]: https://code.claude.com/docs/en/plugins-reference "Plugins reference - Claude Code Docs"
[6]: https://developers.openai.com/codex/guides/agents-md/ "Custom instructions with AGENTS.md – Codex | OpenAI Developers"
[7]: https://cursor.com/docs/plugins?utm_source=chatgpt.com "Plugins | Cursor Docs"
[8]: https://code.claude.com/docs/en/plugins "Create plugins - Claude Code Docs"
[9]: https://developers.openai.com/codex/rules/?utm_source=chatgpt.com "Rules – Codex"
[10]: https://code.claude.com/docs/en/plugin-marketplaces "Create and distribute a plugin marketplace - Claude Code Docs"
