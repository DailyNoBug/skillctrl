import {
  startTransition,
  useDeferredValue,
  useEffect,
  useState,
  type ReactNode
} from "react";
import { locateSkillctrlBinary, runSkillctrl } from "./api";
import type {
  AssetRecord,
  BundleDetail,
  CommandExecution,
  ImportArtifactRecord,
  InstallRecord,
  JsonRecord,
  SourceRecord,
  VerificationResult
} from "./types";

const TARGETS = ["claude-code", "codex", "cursor"] as const;
const SCOPES = ["user", "project"] as const;
const SHELLS = ["bash", "elvish", "fish", "powershell", "zsh"] as const;
const IMPORT_ENDPOINTS = ["claude-code"] as const;

const PAGES = [
  {
    id: "overview",
    title: "总览",
    subtitle: "用更直观的方式管理 skillctrl 的全部工作流。"
  },
  {
    id: "sources",
    title: "源管理",
    subtitle: "管理仓库、认证方式和同步更新。"
  },
  {
    id: "assets",
    title: "资产目录",
    subtitle: "搜索资产、查看元数据并对比支持目标。"
  },
  {
    id: "install",
    title: "安装管理",
    subtitle: "在用户或项目范围安装、卸载资产。"
  },
  {
    id: "status",
    title: "安装状态",
    subtitle: "查看安装记录以及生成的文件。"
  },
  {
    id: "verify",
    title: "一致性校验",
    subtitle: "校验是否已安装、是否最新，以及本地内容是否一致。"
  },
  {
    id: "update",
    title: "更新",
    subtitle: "刷新单个源或整个配置好的资产目录。"
  },
  {
    id: "export",
    title: "导出",
    subtitle: "将资产内容导出到指定目录和格式。"
  },
  {
    id: "import",
    title: "导入",
    subtitle: "扫描已有助手配置并转换成可管理资产。"
  },
  {
    id: "completion",
    title: "命令补全",
    subtitle: "生成 shell 补全脚本，并可一键复制。"
  },
  {
    id: "console",
    title: "控制台",
    subtitle: "查看原始 JSON、stdout、stderr 和执行命令。"
  }
] as const;

type PageId = (typeof PAGES)[number]["id"];
type Target = (typeof TARGETS)[number];
type Scope = (typeof SCOPES)[number];
type Shell = (typeof SHELLS)[number];
type ImportEndpoint = (typeof IMPORT_ENDPOINTS)[number];
type CommandTarget =
  | "sources:list"
  | "source:add"
  | "source:update"
  | "source:remove"
  | "assets:list"
  | "assets:show"
  | "install"
  | "uninstall"
  | "status"
  | "verify"
  | "update"
  | "export"
  | "import:scan"
  | "import:plan"
  | "import:apply"
  | "completion";
type NoticeTone = "success" | "info" | "warning" | "error";

interface Notice {
  tone: NoticeTone;
  message: string;
}

interface BusyCommand {
  label: string;
  startedAt: number;
}

interface SourcesState {
  addName: string;
  addRepo: string;
  addBranch: string;
  addSshKey: string;
  addAccessToken: string;
  updateName: string;
  updateSshKey: string;
  updateAccessToken: string;
  records: SourceRecord[];
  lastJson: string;
}

interface AssetsState {
  source: string;
  target: string;
  search: string;
  showBundleId: string;
  showSource: string;
  records: AssetRecord[];
  detail: BundleDetail | null;
  lastJson: string;
}

interface InstallState {
  bundleId: string;
  source: string;
  target: Target;
  scope: Scope;
  project: string;
  dryRun: boolean;
  uninstallBundleId: string;
  uninstallTarget: Target;
  uninstallScope: Scope;
  uninstallProject: string;
  uninstallDryRun: boolean;
  lastJson: string;
}

interface StatusState {
  target: Target;
  scope: Scope;
  project: string;
  bundle: string;
  records: InstallRecord[];
  lastJson: string;
}

interface VerifyState {
  bundleId: string;
  source: string;
  target: Target;
  scope: Scope;
  project: string;
  result: VerificationResult | null;
  lastJson: string;
}

interface UpdateState {
  source: string;
  lastJson: string;
}

interface ExportState {
  bundleId: string;
  source: string;
  target: string;
  out: string;
  format: string;
  lastJson: string;
}

interface ImportState {
  from: ImportEndpoint;
  path: string;
  bundleId: string;
  out: string;
  scanArtifacts: ImportArtifactRecord[];
  planArtifacts: ImportArtifactRecord[];
  scanJson: string;
  planJson: string;
  applyJson: string;
}

interface CompletionState {
  shell: Shell;
  script: string;
  lastJson: string;
}

function App() {
  const [page, setPage] = useState<PageId>("overview");
  const [busy, setBusy] = useState<BusyCommand | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const [notice, setNotice] = useState<Notice | null>(null);
  const [cliPath, setCliPath] = useState("正在定位 skillctrl 可执行文件...");
  const [overview, setOverview] = useState({
    sourceCount: 0,
    assetCount: 0,
    installationCount: 0
  });
  const [consoleState, setConsoleState] = useState({
    label: "尚未执行命令",
    commandLine: "",
    binaryPath: "",
    stdout: "",
    stderr: "",
    jsonPretty: ""
  });
  const [sources, setSources] = useState<SourcesState>({
    addName: "",
    addRepo: "",
    addBranch: "main",
    addSshKey: "",
    addAccessToken: "",
    updateName: "",
    updateSshKey: "",
    updateAccessToken: "",
    records: [] as SourceRecord[],
    lastJson: ""
  });
  const [assets, setAssets] = useState<AssetsState>({
    source: "",
    target: "",
    search: "",
    showBundleId: "",
    showSource: "",
    records: [] as AssetRecord[],
    detail: null as BundleDetail | null,
    lastJson: ""
  });
  const [installState, setInstallState] = useState<InstallState>({
    bundleId: "",
    source: "",
    target: TARGETS[0],
    scope: SCOPES[0],
    project: "",
    dryRun: false,
    uninstallBundleId: "",
    uninstallTarget: TARGETS[0],
    uninstallScope: SCOPES[0],
    uninstallProject: "",
    uninstallDryRun: false,
    lastJson: ""
  });
  const [statusState, setStatusState] = useState<StatusState>({
    target: TARGETS[0],
    scope: SCOPES[0],
    project: "",
    bundle: "",
    records: [] as InstallRecord[],
    lastJson: ""
  });
  const [verifyState, setVerifyState] = useState<VerifyState>({
    bundleId: "",
    source: "",
    target: TARGETS[0],
    scope: SCOPES[0],
    project: "",
    result: null as VerificationResult | null,
    lastJson: ""
  });
  const [updateState, setUpdateState] = useState<UpdateState>({
    source: "",
    lastJson: ""
  });
  const [exportState, setExportState] = useState<ExportState>({
    bundleId: "",
    source: "",
    target: TARGETS[0],
    out: "",
    format: "native",
    lastJson: ""
  });
  const [importState, setImportState] = useState<ImportState>({
    from: IMPORT_ENDPOINTS[0],
    path: "",
    bundleId: "",
    out: "",
    scanArtifacts: [] as ImportArtifactRecord[],
    planArtifacts: [] as ImportArtifactRecord[],
    scanJson: "",
    planJson: "",
    applyJson: ""
  });
  const [completionState, setCompletionState] = useState<CompletionState>({
    shell: SHELLS[4],
    script: "",
    lastJson: ""
  });

  useEffect(() => {
    let cancelled = false;
    locateSkillctrlBinary()
      .then((path) => {
        if (!cancelled) {
          setCliPath(path);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setCliPath("尚未找到 skillctrl 可执行文件");
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!busy) {
      return undefined;
    }

    const timer = window.setInterval(() => setNow(Date.now()), 120);
    return () => window.clearInterval(timer);
  }, [busy]);

  const deferredAssetFilter = useDeferredValue(assets.search.trim().toLowerCase());
  const filteredAssets = (() => {
    if (!deferredAssetFilter) {
      return assets.records;
    }

    return assets.records.filter((asset) => {
      const haystack = [
        asset.id,
        asset.name,
        asset.source,
        asset.summary,
        asset.version,
        ...asset.asset_types,
        ...asset.targets
      ]
        .join(" ")
        .toLowerCase();

      return haystack.includes(deferredAssetFilter);
    });
  })();

  const currentPageMeta = PAGES.find((item) => item.id === page) ?? PAGES[0];

  async function executeCommand(
    target: CommandTarget,
    label: string,
    args: string[]
  ): Promise<void> {
    if (busy) {
      setNotice({
        tone: "warning",
        message: "还有命令正在执行，请等待当前任务完成后再试。"
      });
      return;
    }

    setBusy({ label, startedAt: Date.now() });
    setNotice({ tone: "info", message: `正在执行：${label}` });

    try {
      const execution = await runSkillctrl(args);
      const payload = execution.json
        ? JSON.stringify(execution.json, null, 2)
        : execution.stdout || execution.stderr;

      setCliPath(execution.binary_path);
      setConsoleState({
        label,
        commandLine: execution.command_line,
        binaryPath: execution.binary_path,
        stdout: execution.stdout,
        stderr: execution.stderr,
        jsonPretty: payload
      });

      applyRawPayload(target, payload);

      if (!execution.success) {
        setNotice({
          tone: "error",
          message: extractErrorMessage(execution)
        });
        return;
      }

      startTransition(() => {
        applySuccessPayload(target, execution.json);
      });
      setNotice({
        tone: "success",
        message: `${label}执行成功。`
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setConsoleState({
        label,
        commandLine: `skillctrl --json-resp ${args.join(" ")}`,
        binaryPath: cliPath,
        stdout: "",
        stderr: message,
        jsonPretty: message
      });
      setNotice({ tone: "error", message });
    } finally {
      setBusy(null);
    }
  }

  function applyRawPayload(target: CommandTarget, payload: string): void {
    switch (target) {
      case "sources:list":
      case "source:add":
      case "source:update":
      case "source:remove":
        setSources((current) => ({ ...current, lastJson: payload }));
        break;
      case "assets:list":
      case "assets:show":
        setAssets((current) => ({ ...current, lastJson: payload }));
        break;
      case "install":
      case "uninstall":
        setInstallState((current) => ({ ...current, lastJson: payload }));
        break;
      case "status":
        setStatusState((current) => ({ ...current, lastJson: payload }));
        break;
      case "verify":
        setVerifyState((current) => ({ ...current, lastJson: payload }));
        break;
      case "update":
        setUpdateState((current) => ({ ...current, lastJson: payload }));
        break;
      case "export":
        setExportState((current) => ({ ...current, lastJson: payload }));
        break;
      case "import:scan":
        setImportState((current) => ({ ...current, scanJson: payload }));
        break;
      case "import:plan":
        setImportState((current) => ({ ...current, planJson: payload }));
        break;
      case "import:apply":
        setImportState((current) => ({ ...current, applyJson: payload }));
        break;
      case "completion":
        setCompletionState((current) => ({ ...current, lastJson: payload }));
        break;
      default:
        break;
    }
  }

  function applySuccessPayload(target: CommandTarget, json: JsonRecord | null): void {
    if (!json) {
      return;
    }

    switch (target) {
      case "sources:list": {
        const records = (json.sources as SourceRecord[] | undefined) ?? [];
        setSources((current) => ({ ...current, records }));
        setOverview((current) => ({ ...current, sourceCount: records.length }));
        break;
      }
      case "assets:list": {
        const records = (json.assets as AssetRecord[] | undefined) ?? [];
        setAssets((current) => ({ ...current, records }));
        setOverview((current) => ({ ...current, assetCount: records.length }));
        break;
      }
      case "assets:show": {
        const detail = (json.bundle as BundleDetail | undefined) ?? null;
        setAssets((current) => ({ ...current, detail }));
        break;
      }
      case "status": {
        const records = (json.installations as InstallRecord[] | undefined) ?? [];
        setStatusState((current) => ({ ...current, records }));
        setOverview((current) => ({
          ...current,
          installationCount: records.length
        }));
        break;
      }
      case "verify": {
        const result = (json.verification as VerificationResult | undefined) ?? null;
        setVerifyState((current) => ({ ...current, result }));
        break;
      }
      case "import:scan": {
        const artifacts = (json.artifacts as ImportArtifactRecord[] | undefined) ?? [];
        setImportState((current) => ({ ...current, scanArtifacts: artifacts }));
        break;
      }
      case "import:plan": {
        const artifacts = (json.artifacts as ImportArtifactRecord[] | undefined) ?? [];
        setImportState((current) => ({ ...current, planArtifacts: artifacts }));
        break;
      }
      case "completion": {
        setCompletionState((current) => ({
          ...current,
          script: typeof json.script === "string" ? json.script : current.script
        }));
        break;
      }
      default:
        break;
    }
  }

  function copyCompletionScript(): void {
    if (!completionState.script.trim()) {
      return;
    }

    navigator.clipboard
      .writeText(completionState.script)
      .then(() => {
        setNotice({
          tone: "success",
          message: "补全脚本已复制到剪贴板。"
        });
      })
      .catch((error) => {
        setNotice({
          tone: "error",
          message: error instanceof Error ? error.message : "复制到剪贴板失败。"
        });
      });
  }

  return (
    <div className="desktop-shell">
      <div className="ambient ambient-left" />
      <div className="ambient ambient-right" />
      <aside className="sidebar">
        <div className="window-dots">
          <span className="dot dot-red" />
          <span className="dot dot-yellow" />
          <span className="dot dot-green" />
        </div>

        <div className="sidebar-brand">
          <div className="sidebar-brand-mark">SC</div>
          <div>
            <div className="sidebar-title">skillctrl-desktop</div>
            <div className="sidebar-copy">原生命令能力，现代化桌面界面。</div>
          </div>
        </div>

        <nav className="sidebar-nav">
          {PAGES.map((item) => (
            <button
              key={item.id}
              className={`nav-item ${page === item.id ? "nav-item-active" : ""}`}
              onClick={() => setPage(item.id)}
              type="button"
            >
              <span className="nav-rail" />
              <span className="nav-text-wrap">
                <span className="nav-label">{item.title}</span>
                <span className="nav-copy">{item.subtitle}</span>
              </span>
            </button>
          ))}
        </nav>

        <div className="sidebar-footer">
          <div className="chip-row">
            <Tag tone="success">已连接 CLI</Tag>
            <Tag tone="mutedDark">Tauri 桌面壳</Tag>
          </div>
          <div className="binary-card">
            <span className="binary-label">当前二进制</span>
            <code>{cliPath}</code>
          </div>
        </div>
      </aside>

      <main className="workspace">
        <header className="topbar card glass-card">
          <div>
            <div className="chip-row">
              <Tag tone="accent">{currentPageMeta.title}</Tag>
              <Tag tone="muted">skillctrl 桌面工作台</Tag>
            </div>
            <h1>{currentPageMeta.title}</h1>
            <p>{currentPageMeta.subtitle}</p>
          </div>
          <div className="topbar-status">
            {busy ? (
              <Tag tone="warning">
                {busy.label} {((now - busy.startedAt) / 1000).toFixed(1)}s
              </Tag>
            ) : (
              <Tag tone="success">就绪</Tag>
            )}
          </div>
        </header>

        {notice ? (
          <section className={`notice notice-${notice.tone}`}>
            <span>{notice.message}</span>
          </section>
        ) : null}

        <section className="content-scroll">
          {page === "overview" ? (
            <>
              <section className="hero card glass-card">
                <div className="chip-row">
                  <Tag tone="accent">桌面工作台</Tag>
                  <Tag tone="neutral">Vite + React + Tauri</Tag>
                </div>
                <h2>用现代化可视界面接管整个 skillctrl 工作流</h2>
                <p>
                  你可以在这里浏览源、查看资产、执行安装、校验差异，并直接查看
                  原始 JSON，无需频繁切回终端。
                </p>
              </section>

              <section className="metric-grid">
                <MetricCard
                  label="已配置源数量"
                  value={String(overview.sourceCount)}
                  caption="由源列表查询结果更新"
                />
                <MetricCard
                  label="当前可见资产"
                  value={String(overview.assetCount)}
                  caption="来自最近一次资产查询"
                />
                <MetricCard
                  label="安装记录"
                  value={String(overview.installationCount)}
                  caption="来自最近一次状态查询"
                />
              </section>

              <Card title="推荐使用流程">
                <ul className="bullet-list">
                  <li>先在“源管理”页面添加仓库，或刷新已有源。</li>
                  <li>再到“资产目录”里搜索目标资产并查看详情。</li>
                  <li>在“安装管理”中安装到用户或项目范围。</li>
                  <li>用“安装状态”和“一致性校验”确认是否安装成功且为最新版本。</li>
                  <li>需要排查时，打开“控制台”查看原始 JSON 和错误输出。</li>
                </ul>
              </Card>
            </>
          ) : null}

          {page === "sources" ? (
            <>
              <Card
                title="已配置源"
                actions={
                  <PrimaryButton
                    onClick={() =>
                      executeCommand("sources:list", "刷新源列表", [
                        "source",
                        "list"
                      ])
                    }
                  >
                    刷新源列表
                  </PrimaryButton>
                }
              >
                {sources.records.length === 0 ? (
                  <EmptyState text="当前还没有载入任何源。你可以先刷新列表，或直接添加新的仓库源。" />
                ) : (
                  <div className="table-wrap">
                    <table className="data-table">
                      <thead>
                        <tr>
                          <th>名称</th>
                          <th>仓库地址</th>
                          <th>分支</th>
                          <th>认证方式</th>
                          <th>操作</th>
                        </tr>
                      </thead>
                      <tbody>
                        {sources.records.map((record) => (
                          <tr key={`${record.name}-${record.repo_url}`}>
                            <td>{record.name}</td>
                            <td className="table-secondary">{record.repo_url}</td>
                            <td>
                              <code>{record.branch}</code>
                            </td>
                            <td>
                              <Tag tone={authTone(record.auth)}>
                                {authLabel(record.auth)}
                              </Tag>
                            </td>
                            <td>
                              <InlineButton
                                onClick={() =>
                                  setSources((current) => ({
                                    ...current,
                                    updateName: record.name
                                  }))
                                }
                              >
                                选中
                              </InlineButton>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </Card>

              <div className="two-col">
                <Card title="添加源">
                  <Field
                    label="名称"
                    value={sources.addName}
                    onChange={(value) =>
                      setSources((current) => ({ ...current, addName: value }))
                    }
                  />
                  <Field
                    label="仓库地址"
                    value={sources.addRepo}
                    onChange={(value) =>
                      setSources((current) => ({ ...current, addRepo: value }))
                    }
                  />
                  <Field
                    label="分支"
                    value={sources.addBranch}
                    onChange={(value) =>
                      setSources((current) => ({ ...current, addBranch: value }))
                    }
                  />
                  <Field
                    label="SSH 私钥路径"
                    value={sources.addSshKey}
                    onChange={(value) =>
                      setSources((current) => ({ ...current, addSshKey: value }))
                    }
                  />
                  <Field
                    label="访问令牌"
                    value={sources.addAccessToken}
                    onChange={(value) =>
                      setSources((current) => ({
                        ...current,
                        addAccessToken: value
                      }))
                    }
                  />
                  <div className="button-row">
                    <PrimaryButton
                      onClick={() => {
                        const args = [
                          "source",
                          "add",
                          sources.addName.trim(),
                          "--repo",
                          sources.addRepo.trim()
                        ];
                        appendIfPresent(args, "--branch", sources.addBranch);
                        appendIfPresent(args, "--ssh-key", sources.addSshKey);
                        appendIfPresent(args, "--access-token", sources.addAccessToken);
                        void executeCommand("source:add", "添加源", args);
                      }}
                    >
                      添加源
                    </PrimaryButton>
                  </div>
                </Card>

                <Card title="更新或删除">
                  <Field
                    label="源名称"
                    value={sources.updateName}
                    onChange={(value) =>
                      setSources((current) => ({ ...current, updateName: value }))
                    }
                  />
                  <Field
                    label="SSH 私钥路径"
                    value={sources.updateSshKey}
                    onChange={(value) =>
                      setSources((current) => ({ ...current, updateSshKey: value }))
                    }
                  />
                  <Field
                    label="访问令牌"
                    value={sources.updateAccessToken}
                    onChange={(value) =>
                      setSources((current) => ({
                        ...current,
                        updateAccessToken: value
                      }))
                    }
                  />
                  <div className="button-row">
                    <PrimaryButton
                      onClick={() => {
                        const args = ["source", "update", sources.updateName.trim()];
                        appendIfPresent(args, "--ssh-key", sources.updateSshKey);
                        appendIfPresent(
                          args,
                          "--access-token",
                          sources.updateAccessToken
                        );
                        void executeCommand("source:update", "更新源", args);
                      }}
                    >
                      更新源
                    </PrimaryButton>
                    <SecondaryButton
                      onClick={() =>
                        executeCommand("source:remove", "删除源", [
                          "source",
                          "remove",
                          sources.updateName.trim()
                        ])
                      }
                    >
                      删除源
                    </SecondaryButton>
                  </div>
                </Card>
              </div>

              <CodeCard title="最近一次源操作返回" value={sources.lastJson} />
            </>
          ) : null}

          {page === "assets" ? (
            <>
              <Card
                title="资产目录"
                actions={
                  <PrimaryButton
                    onClick={() => {
                      const args = ["list"];
                      appendIfPresent(args, "--source", assets.source);
                      appendIfPresent(args, "--target", assets.target);
                      appendIfPresent(args, "--search", assets.search);
                      void executeCommand("assets:list", "查询资产列表", args);
                    }}
                  >
                    加载资产
                  </PrimaryButton>
                }
              >
                <div className="form-grid form-grid-3">
                  <Field
                    label="来源"
                    value={assets.source}
                    onChange={(value) =>
                      setAssets((current) => ({ ...current, source: value }))
                    }
                  />
                  <Field
                    label="目标"
                    value={assets.target}
                    onChange={(value) =>
                      setAssets((current) => ({ ...current, target: value }))
                    }
                  />
                  <Field
                    label="搜索关键词"
                    value={assets.search}
                    onChange={(value) =>
                      setAssets((current) => ({ ...current, search: value }))
                    }
                  />
                </div>
                {filteredAssets.length === 0 ? (
                  <EmptyState text="当前还没有资产数据。先执行一次查询，表格里就会显示结果。" />
                ) : (
                  <div className="table-wrap">
                    <table className="data-table">
                      <thead>
                        <tr>
                          <th>ID</th>
                          <th>类型</th>
                          <th>来源</th>
                          <th>版本</th>
                          <th>摘要</th>
                          <th>查看</th>
                        </tr>
                      </thead>
                      <tbody>
                        {filteredAssets.map((asset) => (
                          <tr key={`${asset.source}-${asset.id}`}>
                            <td>{asset.id}</td>
                            <td>
                              <div className="chip-row chip-row-dense">
                                {asset.asset_types.map((assetType) => (
                                  <Tag key={`${asset.id}-${assetType}`} tone={assetTypeTone(assetType)}>
                                    {assetTypeLabel(assetType)}
                                  </Tag>
                                ))}
                              </div>
                            </td>
                            <td>
                              <Tag tone="neutral">{asset.source}</Tag>
                            </td>
                            <td>
                              <code>{asset.version}</code>
                            </td>
                            <td className="table-secondary">{asset.summary}</td>
                            <td>
                              <InlineButton
                                onClick={() => {
                                  setAssets((current) => ({
                                    ...current,
                                    showBundleId: asset.id,
                                    showSource: asset.source
                                  }));
                                  const args = ["show", asset.id];
                                  appendIfPresent(args, "--source", asset.source);
                                  void executeCommand("assets:show", "查看资产详情", args);
                                }}
                              >
                                查看
                              </InlineButton>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </Card>

              <div className="two-col">
                <Card title="查看资产">
                  <Field
                    label="资产 ID"
                    value={assets.showBundleId}
                    onChange={(value) =>
                      setAssets((current) => ({ ...current, showBundleId: value }))
                    }
                  />
                  <Field
                    label="来源"
                    value={assets.showSource}
                    onChange={(value) =>
                      setAssets((current) => ({ ...current, showSource: value }))
                    }
                  />
                  <div className="button-row">
                    <PrimaryButton
                      onClick={() => {
                        const args = ["show", assets.showBundleId.trim()];
                        appendIfPresent(args, "--source", assets.showSource);
                        void executeCommand("assets:show", "查看资产详情", args);
                      }}
                    >
                      查看详情
                    </PrimaryButton>
                  </div>
                </Card>

                <Card title="当前选中资产">
                  {assets.detail ? (
                    <>
                      <div className="chip-row">
                        <Tag tone="accent">{assets.detail.id}</Tag>
                        <Tag tone="neutral">{assets.detail.source}</Tag>
                        <Tag tone="muted">版本 {assets.detail.version}</Tag>
                        {assets.detail.targets.map((target) => (
                          <Tag key={`${assets.detail?.id}-${target}`} tone="accent">
                            {target}
                          </Tag>
                        ))}
                      </div>
                      <h3 className="detail-title">{assets.detail.name}</h3>
                      {assets.detail.description ? (
                        <p className="detail-copy">{assets.detail.description}</p>
                      ) : null}
                      <div className="detail-stack">
                        {assets.detail.components.map((component) => (
                          <div className="detail-chip-card" key={`${component.kind}-${component.id}`}>
                            <div className="chip-row">
                              <Tag tone="neutral">{component.id}</Tag>
                              <Tag tone={assetTypeTone(component.kind)}>
                                {assetTypeLabel(component.kind)}
                              </Tag>
                              <code>{component.path}</code>
                            </div>
                            {component.description ? (
                              <p className="detail-copy">{component.description}</p>
                            ) : null}
                          </div>
                        ))}
                      </div>
                    </>
                  ) : (
                    <EmptyState text="当前还没有资产详情数据。" />
                  )}
                </Card>
              </div>

              <CodeCard title="最近一次资产操作返回" value={assets.lastJson} />
            </>
          ) : null}

          {page === "install" ? (
            <>
              <div className="two-col">
                <Card title="安装资产">
                  <Field
                    label="资产 ID"
                    value={installState.bundleId}
                    onChange={(value) =>
                      setInstallState((current) => ({ ...current, bundleId: value }))
                    }
                  />
                  <Field
                    label="来源"
                    value={installState.source}
                    onChange={(value) =>
                      setInstallState((current) => ({ ...current, source: value }))
                    }
                  />
                  <SelectField
                    label="目标"
                    value={installState.target}
                    options={TARGETS}
                    onChange={(value) =>
                      setInstallState((current) => ({ ...current, target: value }))
                    }
                  />
                  <SelectField
                    label="范围"
                    value={installState.scope}
                    options={SCOPES}
                    optionLabel={scopeLabel}
                    onChange={(value) =>
                      setInstallState((current) => ({ ...current, scope: value }))
                    }
                  />
                  <Field
                    label="项目路径"
                    value={installState.project}
                    onChange={(value) =>
                      setInstallState((current) => ({ ...current, project: value }))
                    }
                  />
                  <CheckboxField
                    label="仅演练（Dry Run）"
                    checked={installState.dryRun}
                    onChange={(checked) =>
                      setInstallState((current) => ({ ...current, dryRun: checked }))
                    }
                  />
                  <div className="button-row">
                    <PrimaryButton
                      onClick={() => {
                        const args = [
                          "install",
                          installState.bundleId.trim(),
                          "--source",
                          installState.source.trim(),
                          "--target",
                          installState.target,
                          "--scope",
                          installState.scope
                        ];
                        appendIfPresent(args, "--project", installState.project);
                        if (installState.dryRun) {
                          args.push("--dry-run");
                        }
                        void executeCommand("install", "安装资产", args);
                      }}
                    >
                      安装
                    </PrimaryButton>
                  </div>
                </Card>

                <Card title="卸载资产">
                  <Field
                    label="资产 ID"
                    value={installState.uninstallBundleId}
                    onChange={(value) =>
                      setInstallState((current) => ({
                        ...current,
                        uninstallBundleId: value
                      }))
                    }
                  />
                  <SelectField
                    label="目标"
                    value={installState.uninstallTarget}
                    options={TARGETS}
                    onChange={(value) =>
                      setInstallState((current) => ({
                        ...current,
                        uninstallTarget: value
                      }))
                    }
                  />
                  <SelectField
                    label="范围"
                    value={installState.uninstallScope}
                    options={SCOPES}
                    optionLabel={scopeLabel}
                    onChange={(value) =>
                      setInstallState((current) => ({
                        ...current,
                        uninstallScope: value
                      }))
                    }
                  />
                  <Field
                    label="项目路径"
                    value={installState.uninstallProject}
                    onChange={(value) =>
                      setInstallState((current) => ({
                        ...current,
                        uninstallProject: value
                      }))
                    }
                  />
                  <CheckboxField
                    label="仅演练（Dry Run）"
                    checked={installState.uninstallDryRun}
                    onChange={(checked) =>
                      setInstallState((current) => ({
                        ...current,
                        uninstallDryRun: checked
                      }))
                    }
                  />
                  <div className="button-row">
                    <SecondaryButton
                      onClick={() => {
                        const args = [
                          "uninstall",
                          installState.uninstallBundleId.trim(),
                          "--target",
                          installState.uninstallTarget,
                          "--scope",
                          installState.uninstallScope
                        ];
                        appendIfPresent(args, "--project", installState.uninstallProject);
                        if (installState.uninstallDryRun) {
                          args.push("--dry-run");
                        }
                        void executeCommand("uninstall", "卸载资产", args);
                      }}
                    >
                      卸载
                    </SecondaryButton>
                  </div>
                </Card>
              </div>

              <CodeCard
                title="最近一次安装或卸载返回"
                value={installState.lastJson}
              />
            </>
          ) : null}

          {page === "status" ? (
            <>
              <Card
                title="安装状态"
                actions={
                  <PrimaryButton
                    onClick={() => {
                      const args = [
                        "status",
                        "--target",
                        statusState.target,
                        "--scope",
                        statusState.scope
                      ];
                      appendIfPresent(args, "--project", statusState.project);
                      appendIfPresent(args, "--bundle", statusState.bundle);
                      void executeCommand("status", "查询安装状态", args);
                    }}
                  >
                    查询状态
                  </PrimaryButton>
                }
              >
                <div className="form-grid form-grid-4">
                  <SelectField
                    label="目标"
                    value={statusState.target}
                    options={TARGETS}
                    onChange={(value) =>
                      setStatusState((current) => ({ ...current, target: value }))
                    }
                  />
                  <SelectField
                    label="范围"
                    value={statusState.scope}
                    options={SCOPES}
                    optionLabel={scopeLabel}
                    onChange={(value) =>
                      setStatusState((current) => ({ ...current, scope: value }))
                    }
                  />
                  <Field
                    label="项目路径"
                    value={statusState.project}
                    onChange={(value) =>
                      setStatusState((current) => ({ ...current, project: value }))
                    }
                  />
                  <Field
                    label="资产筛选"
                    value={statusState.bundle}
                    onChange={(value) =>
                      setStatusState((current) => ({ ...current, bundle: value }))
                    }
                  />
                </div>
                {statusState.records.length === 0 ? (
                  <EmptyState text="当前还没有安装记录数据。" />
                ) : (
                  <div className="table-wrap">
                    <table className="data-table">
                      <thead>
                        <tr>
                          <th>资产</th>
                          <th>版本</th>
                          <th>来源</th>
                          <th>文件数</th>
                          <th>安装时间</th>
                        </tr>
                      </thead>
                      <tbody>
                        {statusState.records.map((record) => (
                          <tr key={`${record.bundle_id}-${record.installed_at}`}>
                            <td>{record.bundle_id}</td>
                            <td>
                              <code>{record.version}</code>
                            </td>
                            <td>
                              <Tag tone="neutral">{record.source_name ?? "-"}</Tag>
                            </td>
                            <td>
                              <Tag tone="muted">{String(record.files_created.length)}</Tag>
                            </td>
                            <td className="table-secondary">{record.installed_at}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </Card>

              <CodeCard title="最近一次状态查询返回" value={statusState.lastJson} />
            </>
          ) : null}

          {page === "verify" ? (
            <>
              <Card
                title="校验资产"
                actions={
                  <PrimaryButton
                    onClick={() => {
                      const args = [
                        "verify",
                        verifyState.bundleId.trim(),
                        "--target",
                        verifyState.target,
                        "--scope",
                        verifyState.scope
                      ];
                      appendIfPresent(args, "--source", verifyState.source);
                      appendIfPresent(args, "--project", verifyState.project);
                      void executeCommand("verify", "校验资产", args);
                    }}
                  >
                    开始校验
                  </PrimaryButton>
                }
              >
                <div className="form-grid form-grid-5">
                  <Field
                    label="资产 ID"
                    value={verifyState.bundleId}
                    onChange={(value) =>
                      setVerifyState((current) => ({ ...current, bundleId: value }))
                    }
                  />
                  <Field
                    label="来源"
                    value={verifyState.source}
                    onChange={(value) =>
                      setVerifyState((current) => ({ ...current, source: value }))
                    }
                  />
                  <SelectField
                    label="目标"
                    value={verifyState.target}
                    options={TARGETS}
                    onChange={(value) =>
                      setVerifyState((current) => ({ ...current, target: value }))
                    }
                  />
                  <SelectField
                    label="范围"
                    value={verifyState.scope}
                    options={SCOPES}
                    optionLabel={scopeLabel}
                    onChange={(value) =>
                      setVerifyState((current) => ({ ...current, scope: value }))
                    }
                  />
                  <Field
                    label="项目路径"
                    value={verifyState.project}
                    onChange={(value) =>
                      setVerifyState((current) => ({ ...current, project: value }))
                    }
                  />
                </div>
              </Card>

              {verifyState.result ? (
                <>
                  <section className="metric-grid">
                    <MetricCard
                      label="是否已安装"
                      value={yesNo(verifyState.result.installed)}
                      caption="磁盘中是否存在"
                    />
                    <MetricCard
                      label="是否最新版本"
                      value={yesNo(verifyState.result.is_latest_version)}
                      caption={`最新版本：${verifyState.result.latest_version}`}
                    />
                    <MetricCard
                      label="内容是否一致"
                      value={yesNo(verifyState.result.local_matches_source)}
                      caption={`已匹配 ${verifyState.result.files_matching}/${verifyState.result.files_checked} 个文件`}
                    />
                  </section>

                  <Card title="组件校验结果">
                    <div className="table-wrap">
                      <table className="data-table">
                        <thead>
                          <tr>
                            <th>组件</th>
                            <th>类型</th>
                            <th>已安装</th>
                            <th>内容一致</th>
                            <th>详情</th>
                          </tr>
                        </thead>
                        <tbody>
                          {verifyState.result.components.map((component) => (
                            <tr key={`${component.kind}-${component.id}`}>
                              <td>{component.id}</td>
                              <td>
                                <Tag tone={assetTypeTone(component.kind)}>
                                  {assetTypeLabel(component.kind)}
                                </Tag>
                              </td>
                              <td>
                                <Tag tone={component.installed ? "success" : "danger"}>
                                  {yesNo(component.installed)}
                                </Tag>
                              </td>
                              <td>
                                <Tag tone={component.content_matches ? "success" : "danger"}>
                                  {yesNo(component.content_matches)}
                                </Tag>
                              </td>
                              <td className="table-secondary">{component.detail}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  </Card>
                </>
              ) : (
                <Card title="校验摘要">
                  <EmptyState text="执行一次校验后，这里会展示资产级别的校验结果。" />
                </Card>
              )}

              <CodeCard title="最近一次校验返回" value={verifyState.lastJson} />
            </>
          ) : null}

          {page === "update" ? (
            <>
              <Card title="刷新源">
                <Field
                  label="源名称（可选）"
                  value={updateState.source}
                  onChange={(value) =>
                    setUpdateState((current) => ({ ...current, source: value }))
                  }
                />
                <div className="button-row">
                  <PrimaryButton
                    onClick={() => {
                      const args = ["update"];
                      if (updateState.source.trim()) {
                        args.push(updateState.source.trim());
                      }
                      void executeCommand("update", "更新源", args);
                    }}
                  >
                    更新指定源
                  </PrimaryButton>
                  <SecondaryButton
                    onClick={() => executeCommand("update", "更新全部源", ["update"])}
                  >
                    更新全部
                  </SecondaryButton>
                </div>
              </Card>

              <CodeCard title="最近一次更新返回" value={updateState.lastJson} />
            </>
          ) : null}

          {page === "export" ? (
            <>
              <Card title="导出资产">
                <Field
                  label="资产 ID"
                  value={exportState.bundleId}
                  onChange={(value) =>
                    setExportState((current) => ({ ...current, bundleId: value }))
                  }
                />
                <Field
                  label="来源"
                  value={exportState.source}
                  onChange={(value) =>
                    setExportState((current) => ({ ...current, source: value }))
                  }
                />
                <Field
                  label="目标"
                  value={exportState.target}
                  onChange={(value) =>
                    setExportState((current) => ({ ...current, target: value }))
                  }
                />
                <Field
                  label="输出目录"
                  value={exportState.out}
                  onChange={(value) =>
                    setExportState((current) => ({ ...current, out: value }))
                  }
                />
                <Field
                  label="格式"
                  value={exportState.format}
                  onChange={(value) =>
                    setExportState((current) => ({ ...current, format: value }))
                  }
                />
                <div className="button-row">
                  <PrimaryButton
                    onClick={() =>
                      executeCommand("export", "导出资产", [
                        "export",
                        exportState.bundleId.trim(),
                        "--source",
                        exportState.source.trim(),
                        "--target",
                        exportState.target.trim(),
                        "--out",
                        exportState.out.trim(),
                        "--format",
                        exportState.format.trim()
                      ])
                    }
                  >
                    导出
                  </PrimaryButton>
                </div>
              </Card>

              <CodeCard title="最近一次导出返回" value={exportState.lastJson} />
            </>
          ) : null}

          {page === "import" ? (
            <>
              <div className="three-col">
                <Card title="导入扫描">
                  <SelectField
                    label="来源端"
                    value={importState.from}
                    options={IMPORT_ENDPOINTS}
                    onChange={(value) =>
                      setImportState((current) => ({ ...current, from: value }))
                    }
                  />
                  <Field
                    label="路径"
                    value={importState.path}
                    onChange={(value) =>
                      setImportState((current) => ({ ...current, path: value }))
                    }
                  />
                  <div className="button-row">
                    <PrimaryButton
                      onClick={() =>
                        executeCommand("import:scan", "扫描导入源", [
                          "import",
                          "scan",
                          "--from",
                          importState.from,
                          "--path",
                          importState.path.trim()
                        ])
                    }
                  >
                      扫描
                    </PrimaryButton>
                  </div>
                  {importState.scanArtifacts.length === 0 ? (
                    <EmptyState text="当前还没有扫描结果。" />
                  ) : (
                    <ArtifactList artifacts={importState.scanArtifacts} />
                  )}
                </Card>

                <Card title="导入规划">
                  <SelectField
                    label="来源端"
                    value={importState.from}
                    options={IMPORT_ENDPOINTS}
                    onChange={(value) =>
                      setImportState((current) => ({ ...current, from: value }))
                    }
                  />
                  <Field
                    label="路径"
                    value={importState.path}
                    onChange={(value) =>
                      setImportState((current) => ({ ...current, path: value }))
                    }
                  />
                  <Field
                    label="资产 ID"
                    value={importState.bundleId}
                    onChange={(value) =>
                      setImportState((current) => ({ ...current, bundleId: value }))
                    }
                  />
                  <div className="button-row">
                    <PrimaryButton
                      onClick={() => {
                        const args = [
                          "import",
                          "plan",
                          "--from",
                          importState.from,
                          "--path",
                          importState.path.trim()
                        ];
                        appendIfPresent(args, "--id", importState.bundleId);
                        void executeCommand("import:plan", "生成导入计划", args);
                      }}
                    >
                      生成计划
                    </PrimaryButton>
                  </div>
                  {importState.planArtifacts.length === 0 ? (
                    <EmptyState text="当前还没有导入计划。" />
                  ) : (
                    <ArtifactList artifacts={importState.planArtifacts} />
                  )}
                </Card>

                <Card title="执行导入">
                  <SelectField
                    label="来源端"
                    value={importState.from}
                    options={IMPORT_ENDPOINTS}
                    onChange={(value) =>
                      setImportState((current) => ({ ...current, from: value }))
                    }
                  />
                  <Field
                    label="路径"
                    value={importState.path}
                    onChange={(value) =>
                      setImportState((current) => ({ ...current, path: value }))
                    }
                  />
                  <Field
                    label="输出目录"
                    value={importState.out}
                    onChange={(value) =>
                      setImportState((current) => ({ ...current, out: value }))
                    }
                  />
                  <div className="button-row">
                    <PrimaryButton
                      onClick={() =>
                        executeCommand("import:apply", "执行导入", [
                          "import",
                          "apply",
                          "--from",
                          importState.from,
                          "--path",
                          importState.path.trim(),
                          "--out",
                          importState.out.trim()
                        ])
                    }
                  >
                      执行导入
                    </PrimaryButton>
                  </div>
                  <p className="detail-copy">
                    执行导入后的原始 JSON 会显示在下方响应面板中。
                  </p>
                </Card>
              </div>

              <CodeCard title="最近一次扫描返回" value={importState.scanJson} />
              <CodeCard title="最近一次规划返回" value={importState.planJson} />
              <CodeCard title="最近一次导入返回" value={importState.applyJson} />
            </>
          ) : null}

          {page === "completion" ? (
            <>
              <Card title="Shell 补全">
                <SelectField
                  label="Shell 类型"
                  value={completionState.shell}
                  options={SHELLS}
                  onChange={(value) =>
                    setCompletionState((current) => ({ ...current, shell: value }))
                  }
                />
                <div className="button-row">
                  <PrimaryButton
                    onClick={() =>
                      executeCommand("completion", "生成补全脚本", [
                        "completion",
                        completionState.shell
                      ])
                    }
                  >
                    生成脚本
                  </PrimaryButton>
                  <SecondaryButton onClick={copyCompletionScript}>
                    复制脚本
                  </SecondaryButton>
                </div>
              </Card>

              <CodeCard title="生成的脚本" value={completionState.script} />
              <CodeCard
                title="最近一次补全返回"
                value={completionState.lastJson}
              />
            </>
          ) : null}

          {page === "console" ? (
            <>
              <Card title="最近一次命令">
                <div className="console-stack">
                  <div className="chip-row">
                    <Tag tone="accent">{consoleState.label}</Tag>
                    {consoleState.binaryPath ? (
                      <Tag tone="muted">{consoleState.binaryPath}</Tag>
                    ) : null}
                  </div>
                  <code className="command-line">
                    {consoleState.commandLine || "当前还没有执行过命令。"}
                  </code>
                </div>
              </Card>

              <CodeCard title="JSON 或 stdout" value={consoleState.jsonPretty} />
              <CodeCard title="stderr 输出" value={consoleState.stderr} />
            </>
          ) : null}
        </section>
      </main>
    </div>
  );
}

function appendIfPresent(args: string[], flag: string, value: string): void {
  if (value.trim()) {
    args.push(flag, value.trim());
  }
}

function yesNo(value: boolean): string {
  return value ? "是" : "否";
}

function extractErrorMessage(execution: CommandExecution): string {
  if (execution.json?.error && typeof execution.json.error === "string") {
    return execution.json.error;
  }

  if (execution.stderr.trim()) {
    return execution.stderr.trim();
  }

  if (execution.stdout.trim()) {
    return execution.stdout.trim();
  }

  return "skillctrl 命令执行失败，但没有返回可读的错误信息。";
}

function authTone(auth: string): TagTone {
  const lowered = auth.toLowerCase();
  if (lowered.includes("ssh")) {
    return "accent";
  }
  if (lowered.includes("token") || lowered.includes("https")) {
    return "warning";
  }
  return "neutral";
}

function authLabel(auth: string): string {
  const lowered = auth.toLowerCase();
  if (lowered.includes("ssh")) {
    return "SSH 密钥";
  }
  if (lowered.includes("token")) {
    return "访问令牌";
  }
  if (lowered.includes("https")) {
    return "HTTPS";
  }
  if (lowered.includes("none") || lowered.includes("anonymous")) {
    return "无认证";
  }
  if (lowered.includes("local")) {
    return "本地";
  }
  return auth;
}

function assetTypeTone(kind: string): TagTone {
  if (kind === "skill" || kind === "agent") {
    return "accent";
  }
  if (kind === "rule" || kind === "hook") {
    return "warning";
  }
  if (kind === "mcp" || kind === "resource" || kind === "command") {
    return "neutral";
  }
  return "muted";
}

function assetTypeLabel(kind: string): string {
  switch (kind) {
    case "skill":
      return "技能";
    case "rule":
      return "规则";
    case "mcp":
      return "MCP";
    case "resource":
      return "资源";
    case "agent":
      return "代理";
    case "command":
      return "命令";
    case "hook":
      return "钩子";
    default:
      return kind;
  }
}

function scopeLabel(scope: string): string {
  if (scope === "user") {
    return "用户";
  }
  if (scope === "project") {
    return "项目";
  }
  return scope;
}

type TagTone =
  | "accent"
  | "neutral"
  | "muted"
  | "mutedDark"
  | "success"
  | "warning"
  | "danger";

function Tag(props: { children: ReactNode; tone: TagTone }) {
  return <span className={`tag tag-${props.tone}`}>{props.children}</span>;
}

function Card(props: {
  title: string;
  children: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <section className="card content-card">
      <header className="card-header">
        <div className="card-title-wrap">
          <span className="card-rail" />
          <h3>{props.title}</h3>
        </div>
        {props.actions ? <div className="card-actions">{props.actions}</div> : null}
      </header>
      <div className="card-body">{props.children}</div>
    </section>
  );
}

function CodeCard(props: { title: string; value: string }) {
  return (
    <section className="card content-card">
      <header className="card-header">
        <div className="card-title-wrap">
          <span className="card-rail" />
          <h3>{props.title}</h3>
        </div>
        <div className="chip-row">
          <Tag tone="mutedDark">JSON / 日志</Tag>
          <Tag tone="mutedDark">只读</Tag>
        </div>
      </header>
      <div className="code-surface">
        <pre>{props.value.trim() ? props.value : "暂无返回内容。"}</pre>
      </div>
    </section>
  );
}

function MetricCard(props: {
  label: string;
  value: string;
  caption: string;
}) {
  return (
    <section className="card metric-card">
      <Tag tone="muted">{props.label}</Tag>
      <strong>{props.value}</strong>
      <span>{props.caption}</span>
    </section>
  );
}

function Field(props: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="field">
      <span>{props.label}</span>
      <input
        value={props.value}
        onChange={(event) => props.onChange(event.target.value)}
      />
    </label>
  );
}

function SelectField<T extends string>(props: {
  label: string;
  value: T;
  options: readonly T[];
  onChange: (value: T) => void;
  optionLabel?: (value: T) => string;
}) {
  return (
    <label className="field">
      <span>{props.label}</span>
      <select
        value={props.value}
        onChange={(event) => props.onChange(event.target.value as T)}
      >
        {props.options.map((option) => (
          <option key={option} value={option}>
            {props.optionLabel ? props.optionLabel(option) : option}
          </option>
        ))}
      </select>
    </label>
  );
}

function CheckboxField(props: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="checkbox-field">
      <input
        checked={props.checked}
        type="checkbox"
        onChange={(event) => props.onChange(event.target.checked)}
      />
      <span>{props.label}</span>
    </label>
  );
}

function PrimaryButton(props: {
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button className="button button-primary" onClick={props.onClick} type="button">
      {props.children}
    </button>
  );
}

function SecondaryButton(props: {
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button className="button button-secondary" onClick={props.onClick} type="button">
      {props.children}
    </button>
  );
}

function InlineButton(props: {
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button className="button button-inline" onClick={props.onClick} type="button">
      {props.children}
    </button>
  );
}

function EmptyState(props: { text: string }) {
  return <div className="empty-state">{props.text}</div>;
}

function ArtifactList(props: { artifacts: ImportArtifactRecord[] }) {
  return (
    <div className="artifact-list">
      {props.artifacts.map((artifact) => (
        <div className="detail-chip-card" key={`${artifact.kind}-${artifact.id}-${artifact.path}`}>
          <div className="chip-row">
            <Tag tone={assetTypeTone(artifact.kind)}>
              {assetTypeLabel(artifact.kind)}
            </Tag>
            <Tag tone="neutral">{artifact.id ?? "-"}</Tag>
          </div>
          <code>{artifact.path}</code>
          {artifact.description ? <p className="detail-copy">{artifact.description}</p> : null}
        </div>
      ))}
    </div>
  );
}

export default App;
