# skillctrl 架构文档

## 概览

skillctrl 采用模块化、可扩展的架构设计，核心是一个统一的 **Canonical Model（规范模型）**，通过 **Adapter（适配器）** 模式支持多个 AI 编程助手。

## 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI Layer                           │
│                    (skillctrl-cli)                         │
│  Commands: source, list, install, uninstall, import, export  │
└────────────────────────┬────────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        ▼                ▼                ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│   Catalog    │ │    State     │ │     Git      │
│   Parser     │ │  Management  │ │   Operations  │
└──────────────┘ └──────────────┘ └──────────────┘
        │                │                │
        └────────────────┼────────────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        ▼                ▼                ▼
┌─────────────────────────────────────────────────────────────┐
│                      Core Traits                            │
│  Component | Adapter | Importer | Exporter                  │
└────────────────────────┬────────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        ▼                ▼                ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│   Claude     │ │    Codex     │ │   Cursor     │
│   Adapter    │ │   Adapter    │ │   Adapter    │
└──────────────┘ └──────────────┘ └──────────────┘
```

## 核心抽象

### 1. Component（组件）

所有可安装内容的抽象表示。

```rust
pub trait Component: Any + Send + Sync {
    fn kind(&self) -> ComponentKind;
    fn id(&self) -> &str;
    fn validate(&self) -> ValidationReport;
    fn dependencies(&self) -> &[ComponentDependency];
}
```

**组件类型**：
- `Skill` - AI 能力/技能
- `Rule` - 行为规则
- `Command` - 命令
- `McpServer` - MCP 服务器
- `Hook` - 生命周期钩子
- `Resource` - 资源文件
- `Agent` - AI 代理

### 2. Adapter（适配器）

将规范模型转换为特定 AI 软件格式的接口。

```rust
pub trait Adapter {
    fn endpoint(&self) -> Endpoint;
    fn capabilities(&self) -> AdapterCapabilities;
    fn pre_install(&self, ctx: &InstallContext) -> Result<HookResult>;
    fn post_install(&self, result: &InstallResult) -> Result<HookResult>;
}

pub trait InstallAdapter: Adapter {
    async fn plan_install(&self, bundle: &BundleManifest, ctx: &InstallContext)
        -> Result<InstallPlan>;
    async fn apply_install(&self, plan: &InstallPlan) -> Result<InstallResult>;
}
```

**适配器职责**：
1. 解析 bundle manifest
2. 规划文件安装位置
3. 处理格式转换
4. 执行文件操作
5. 记录安装状态

### 3. Importer（导入器）

从现有配置扫描并转换为规范格式的接口。

```rust
pub trait Importer {
    fn endpoint(&self) -> Endpoint;
    async fn scan(&self, req: &ScanRequest) -> Result<DetectedArtifacts>;
    async fn plan_import(&self, req: &ImportRequest, artifacts: &DetectedArtifacts>)
        -> Result<ImportPlan>;
    async fn apply_import(&self, req: &ApplyImportRequest) -> Result<ImportResult>;
}
```

**导入器职责**：
1. 扫描配置目录
2. 识别组件类型
3. 提取组件内容
4. 生成 bundle manifest
5. 写入规范化目录

### 4. Exporter（导出器）

将 bundle 导出为原生 marketplace 格式。

```rust
pub trait Exporter {
    fn endpoint(&self) -> Endpoint;
    fn supported_formats(&self) -> Vec<ExportFormat>;
    async fn plan_export(&self, req: &ExportRequest) -> Result<ExportPlan>;
    async fn apply_export(&self, plan: &ExportPlan) -> Result<ExportResult>;
}
```

## 数据流

### 安装流程

```
┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐
│  CLI    │───▶│ Catalog │───▶│ Adapter │───▶│  Files  │
│ Command │    │   Load  │    │  Plan   │    │  Write  │
└─────────┘    └─────────┘    └─────────┘    └─────────┘
                     │                               │
                     ▼                               │
              ┌─────────┐                          │
              │ Bundle  │                          │
              │ Manifest│                          │
              └─────────┘                          │
                     │                               │
                     └───────────────────────────────┘
                               │
                               ▼
                        ┌─────────┐
                        │  State  │
                        │ Record  │
                        └─────────┘
```

### 导入流程

```
┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐
│  CLI    │───▶│Importer │───▶│Scanner  │───▶│ Artifacts│
│ Command │    │   Plan  │    │         │    │ Detected│
└─────────┘    └─────────┘    └─────────┘    └─────────┘
                     │                               │
                     ▼                               ▼
              ┌─────────┐                    ┌─────────┐
              │  Bundle │                    │ Components│
              │ Generate│                    │  Copy   │
              └─────────┘                    └─────────┘
```

## 目录映射

### Claude Code

```
canonical/           .claude/
├── skills/          ├── skills/
│   └── {id}/        │   └── {id}/
│       └── SKILL.md │       └── SKILL.md
├── rules/           ├── rules/
│   └── {id}.md      │   └── {id}.md
├── commands/        ├── commands/
│   └── {id}.md      │   └── {id}.md
├── agents/          ├── agents/
│   └── {id}/        │   └── {id}/
├── hooks/           ├── settings.json (merged)
└── mcp/             └── .mcp.json (merged)
    └── {id}.json
```

### Codex

```
canonical/           .codex/
├── skills/          ├── skills/
│   └── {id}/        │   └── {id}/
│       └── SKILL.md │       └── SKILL.md
├── rules/           ├── AGENTS.md (merged)
│   └── {id}.md
└── mcp/             └── config.toml (merged)
    └── {id}.json    (mcpServers section)
```

### Cursor

```
canonical/           .cursor/
├── rules/           ├── rules/
│   └── {id}.md      │   └── {id}.mdc
└── skills/          (converted to .mdc rules)
    └── {id}/
```

## 状态管理

使用 SQLite 持久化：

```sql
-- 源配置
CREATE TABLE sources (
    name TEXT PRIMARY KEY,
    repo_url TEXT NOT NULL,
    branch TEXT NOT NULL,
    cache_path TEXT NOT NULL,
    last_commit TEXT,
    updated_at TEXT
);

-- 安装记录
CREATE TABLE installations (
    id INTEGER PRIMARY KEY,
    bundle_id TEXT NOT NULL,
    bundle_version TEXT NOT NULL,
    source_name TEXT,
    endpoint TEXT NOT NULL,
    scope TEXT NOT NULL,
    project_path TEXT,
    installed_at TEXT NOT NULL,
    files_created TEXT NOT NULL,
    backup_path TEXT,
    UNIQUE(bundle_id, endpoint, scope, project_path)
);

-- 文件记录
CREATE TABLE files (
    id INTEGER PRIMARY KEY,
    installation_id INTEGER NOT NULL,
    path TEXT NOT NULL,
    original_hash TEXT,
    FOREIGN KEY(installation_id) REFERENCES installations(id)
);
```

## 扩展点

### 添加新 AI 软件

1. 创建 `skillctrl-adapter-{name}` crate
2. 实现 `Adapter`、`InstallAdapter` traits
3. 创建 `skillctrl-importer-{name}` crate
4. 实现 `Importer` trait
5. 在 CLI 中注册

### 添加新组件类型

1. 扩展 `ComponentKind` enum
2. 更新各 adapter 的 `capabilities()`
3. 实现安装/导入逻辑

### 添加中间件

```rust
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
```

示例中间件：
- `ConflictDetectionMiddleware`
- `BackupMiddleware`
- `ValidationMiddleware`
- `LoggingMiddleware`

## 性能考虑

### 异步 I/O

所有文件操作使用 `tokio::fs` 或 `spawn_blocking`：

```rust
let content = tokio::task::spawn_blocking(move || {
    std::fs::read_to_string(path)
}).await??;
```

### 缓存

- Git 仓库缓存（按需 fetch）
- Bundle 解析缓存（内存）
- Catalog 索引（未来）

### 并行处理

```rust
use futures::future::join_all;

let results = join_all(
    bundles.iter().map(|b| self.install_bundle(b))
).await;
```

## 错误处理

使用统一的 `Error` 类型：

```rust
pub enum Error {
    Io(String),
    Serialization(String),
    Git(String),
    Database(String),
    Validation(String),
    NotFound(String),
    Conflict(String),
    // ...
}
```

## 安全考虑

1. **输入验证**：所有用户输入都经过验证
2. **路径安全**：使用 `camino::Utf8Path` 防止路径遍历
3. **沙箱**：支持 sandbox 模式（Codex rules）
4. **备份**：安装前自动备份
5. **签名**：未来支持 bundle 签名验证

## 测试策略

### 单元测试

每个 crate 的 `tests/` 模块：

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_component_kind() {
        assert_eq!(ComponentKind::Skill.to_string(), "skill");
    }
}
```

### 集成测试

`examples/` 中的示例 catalog 和 bundles

### 端到端测试

未来使用 `tempfile` 创建临时目录进行测试

## 未来规划

### Phase 1 (当前) ✅
- 基础 CLI
- Claude/Codex/Cursor adapters
- 导入/导出框架

### Phase 2 (下一期)
- TUI 界面（`ratatui`）
- 插件系统
- 中间件系统
- 事件系统

### Phase 3
- GUI 桌面应用（`tauri`）
- Web API（`axum`）
- 云端同步
- 团队协作

## 贡献

欢迎贡献！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详情。

## 许可证

Apache-2.0 OR MIT
