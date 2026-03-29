//! skillctrl - Unified skills marketplace for Claude Code, Codex, and Cursor.

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use skillctrl_catalog::{SourceBundle, SourceCatalog};
use skillctrl_core::{ComponentKind, Endpoint, KnownEndpoint, Scope};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

/// skillctrl - Unified skills marketplace for AI coding assistants
#[derive(Parser)]
#[command(name = "skillctrl")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Manage skills, rules, and components across Claude Code, Codex, and Cursor", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Disable progress bars
    #[arg(short, long)]
    quiet: bool,

    /// Return a structured JSON response
    #[arg(long, global = true)]
    json_resp: bool,
}

/// Available commands
#[derive(Subcommand)]
enum Commands {
    /// Manage sources (git repositories containing bundles)
    Source {
        #[command(subcommand)]
        action: SourceCommands,
    },

    /// List available bundles
    List {
        /// Source name
        #[arg(short = 'S', long)]
        source: Option<String>,

        /// Filter by endpoint
        #[arg(short, long)]
        target: Option<String>,

        /// Search query
        #[arg(short = 's', long)]
        search: Option<String>,
    },

    /// Show bundle details
    Show {
        /// Bundle ID
        bundle_id: String,

        /// Source name
        #[arg(short = 'S', long)]
        source: Option<String>,
    },

    /// Install a bundle
    Install {
        /// Bundle ID
        bundle_id: String,

        /// Source name
        #[arg(short = 'S', long)]
        source: String,

        /// Target endpoint
        #[arg(short, long)]
        target: String,

        /// Installation scope
        #[arg(short, long)]
        scope: String,

        /// Project path (for project scope)
        #[arg(short, long)]
        project: Option<PathBuf>,

        /// Dry run (don't make changes)
        #[arg(long)]
        dry_run: bool,
    },

    /// Uninstall a bundle
    Uninstall {
        /// Bundle ID
        bundle_id: String,

        /// Target endpoint
        #[arg(short, long)]
        target: String,

        /// Installation scope
        #[arg(short, long)]
        scope: String,

        /// Project path (for project scope)
        #[arg(short, long)]
        project: Option<PathBuf>,

        /// Dry run (don't make changes)
        #[arg(long)]
        dry_run: bool,
    },

    /// Import existing configuration
    Import {
        #[command(subcommand)]
        action: ImportCommands,
    },

    /// Show installation status
    Status {
        /// Target endpoint
        #[arg(short, long)]
        target: String,

        /// Installation scope
        #[arg(short, long)]
        scope: String,

        /// Project path (for project scope)
        #[arg(short, long)]
        project: Option<PathBuf>,

        /// Bundle ID (optional)
        #[arg(short, long)]
        bundle: Option<String>,
    },

    /// Update sources
    Update {
        /// Source name (leave empty to update all)
        source: Option<String>,
    },

    /// Export bundle to native format
    Export {
        /// Bundle ID
        bundle_id: String,

        /// Source name
        #[arg(short = 'S', long)]
        source: String,

        /// Target endpoint/format
        #[arg(short, long)]
        target: String,

        /// Output directory
        #[arg(short, long)]
        out: PathBuf,

        /// Export format
        #[arg(short, long)]
        format: String,
    },

    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completion for
        shell: Shell,
    },

    /// Verify whether a bundle is installed and matches the latest source content
    Verify {
        /// Bundle ID
        bundle_id: String,

        /// Source name
        #[arg(short = 'S', long)]
        source: Option<String>,

        /// Target endpoint
        #[arg(short, long)]
        target: String,

        /// Installation scope
        #[arg(short, long)]
        scope: String,

        /// Project path (for project scope)
        #[arg(short, long)]
        project: Option<PathBuf>,
    },
}

/// Source management commands
#[derive(Subcommand)]
enum SourceCommands {
    /// Add a new source
    Add {
        /// Source name
        name: String,

        /// Git repository URL
        #[arg(short, long)]
        repo: String,

        /// Branch name
        #[arg(short, long, default_value = "main")]
        branch: String,

        /// SSH private key to use for SSH repositories
        #[arg(long, value_name = "PATH", conflicts_with = "access_token")]
        ssh_key: Option<PathBuf>,

        /// Access token to use for HTTPS repositories
        #[arg(long, value_name = "TOKEN", conflicts_with = "ssh_key")]
        access_token: Option<String>,
    },

    /// List all sources
    List,

    /// Remove a source
    Remove {
        /// Source name
        name: String,
    },

    /// Update a source
    Update {
        /// Source name
        name: String,

        /// SSH private key to use for SSH repositories
        #[arg(long, value_name = "PATH", conflicts_with = "access_token")]
        ssh_key: Option<PathBuf>,

        /// Access token to use for HTTPS repositories
        #[arg(long, value_name = "TOKEN", conflicts_with = "ssh_key")]
        access_token: Option<String>,
    },
}

/// Import commands
#[derive(Subcommand)]
enum ImportCommands {
    /// Scan for artifacts
    Scan {
        /// Source endpoint
        #[arg(short, long)]
        from: String,

        /// Path to scan
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Create import plan
    Plan {
        /// Source endpoint
        #[arg(short, long)]
        from: String,

        /// Source path
        #[arg(short, long)]
        path: PathBuf,

        /// Bundle ID (optional)
        #[arg(short, long)]
        id: Option<String>,
    },

    /// Apply import plan
    Apply {
        /// Source endpoint
        #[arg(short, long)]
        from: String,

        /// Source path
        #[arg(short, long)]
        path: PathBuf,

        /// Output directory
        #[arg(short, long)]
        out: PathBuf,
    },
}

#[derive(Clone, Copy)]
struct OutputConfig {
    json: bool,
    quiet: bool,
}

#[derive(Serialize)]
struct ErrorResponse<'a> {
    success: bool,
    error: &'a str,
}

#[derive(Debug, Clone, Serialize)]
struct SourceJsonView {
    name: String,
    repo_url: String,
    branch: String,
    auth: String,
    last_commit: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SourceUpdateView {
    name: String,
    repo_url: String,
    branch: String,
    auth: String,
    last_commit: Option<String>,
    bundles: usize,
}

#[derive(Debug, Clone, Serialize)]
struct AssetListRow {
    id: String,
    name: String,
    source: String,
    version: String,
    asset_types: Vec<String>,
    targets: Vec<String>,
    summary: String,
}

#[derive(Debug, Clone, Serialize)]
struct InstallRecordView {
    bundle_id: String,
    version: String,
    source_name: Option<String>,
    endpoint: String,
    scope: String,
    project_path: Option<String>,
    installed_at: String,
    files_created: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct VerifyFileView {
    path: String,
    exists: bool,
    matches_expected: bool,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
struct VerifyComponentView {
    id: String,
    kind: String,
    installed: bool,
    content_matches: bool,
    detail: String,
    files: Vec<VerifyFileView>,
}

#[derive(Debug, Clone, Serialize)]
struct VerifyBundleView {
    bundle_id: String,
    source: String,
    target: String,
    scope: String,
    project_path: Option<String>,
    installed: bool,
    installed_version: Option<String>,
    latest_version: String,
    is_latest_version: bool,
    local_matches_source: bool,
    installation_record_found: bool,
    files_checked: usize,
    files_matching: usize,
    components: Vec<VerifyComponentView>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let output = OutputConfig {
        json: cli.json_resp,
        quiet: cli.quiet,
    };

    // Initialize tracing
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(if cli.verbose {
                tracing::Level::DEBUG.into()
            } else {
                tracing::Level::INFO.into()
            }),
        )
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("failed to set tracing subscriber");

    // Run command
    match run_command(cli).await {
        Ok(_) => Ok(()),
        Err(e) => {
            if output.json {
                let payload = ErrorResponse {
                    success: false,
                    error: &e.to_string(),
                };
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                eprintln!("\x1b[31merror:\x1b[0m {}", e);
            }
            std::process::exit(1);
        }
    }
}

async fn run_command(cli: Cli) -> Result<()> {
    let output = OutputConfig {
        json: cli.json_resp,
        quiet: cli.quiet,
    };

    match cli.command {
        Commands::Source { action } => handle_source_command(action, output).await,
        Commands::List {
            source,
            target,
            search,
        } => handle_list(source, target, search, output).await,
        Commands::Show { bundle_id, source } => handle_show(bundle_id, source, output).await,
        Commands::Install {
            bundle_id,
            source,
            target,
            scope,
            project,
            dry_run,
        } => handle_install(bundle_id, source, target, scope, project, dry_run, output).await,
        Commands::Uninstall {
            bundle_id,
            target,
            scope,
            project,
            dry_run,
        } => handle_uninstall(bundle_id, target, scope, project, dry_run, output).await,
        Commands::Import { action } => handle_import_command(action, output).await,
        Commands::Status {
            target,
            scope,
            project,
            bundle,
        } => handle_status(target, scope, project, bundle, output).await,
        Commands::Update { source } => handle_update(source, output).await,
        Commands::Export {
            bundle_id,
            source,
            target,
            out,
            format,
        } => handle_export(bundle_id, source, target, out, format, output).await,
        Commands::Completion { shell } => handle_completion(shell, output),
        Commands::Verify {
            bundle_id,
            source,
            target,
            scope,
            project,
        } => handle_verify(bundle_id, source, target, scope, project, output).await,
    }
}

async fn handle_source_command(action: SourceCommands, output: OutputConfig) -> Result<()> {
    match action {
        SourceCommands::Add {
            name,
            repo,
            branch,
            ssh_key,
            access_token,
        } => source_add(name, repo, branch, ssh_key, access_token, output).await,
        SourceCommands::List => source_list(output).await,
        SourceCommands::Remove { name } => source_remove(name, output).await,
        SourceCommands::Update {
            name,
            ssh_key,
            access_token,
        } => source_update(name, ssh_key, access_token, output).await,
    }
}

async fn source_add(
    name: String,
    repo: String,
    branch: String,
    ssh_key: Option<PathBuf>,
    access_token: Option<String>,
    output: OutputConfig,
) -> Result<()> {
    validate_source_auth_args(&repo, ssh_key.as_ref(), access_token.as_deref())?;
    if !output.json && !output.quiet {
        println!("Adding source '{}' from {}", name, repo);
    }

    // Get cache directory
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".cache"))
        .join("skillctrl");
    std::fs::create_dir_all(&cache_dir)?;

    let source = build_git_source(
        name.clone(),
        repo.clone(),
        branch.clone(),
        cache_dir.clone(),
        ssh_key,
        access_token,
    );

    // Clone the repository
    let spinner = create_spinner("Cloning repository...".to_string(), output);
    let git_manager = skillctrl_git::GitManager::new(cache_dir);
    let path = git_manager.clone(&source).await?;
    let catalog = SourceCatalog::load_from_dir(&path)
        .with_context(|| format!("failed to load source catalog from {}", path.display()))?;
    let current_commit = git_manager.current_commit(&source).await.ok();
    spinner.finish_with_message(format!(
        "Repository cloned successfully ({} bundles)",
        catalog.bundles().len()
    ));

    let state = skillctrl_state::StateManager::open_default().await?;
    state.register_source(&source).await?;
    state
        .update_source_sync_status(&name, current_commit.as_deref())
        .await?;

    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "action": "source_add",
            "source": {
                "name": name,
                "repo_url": repo,
                "branch": branch,
                "auth": auth_label_from_args(source.ssh_key_path.as_ref(), source.access_token.as_deref(), &source.repo_url),
                "last_commit": current_commit,
            }
        }))?;
    } else {
        println!("✓ Source '{}' added successfully", name);
    }
    Ok(())
}

async fn source_list(output: OutputConfig) -> Result<()> {
    let state = skillctrl_state::StateManager::open_default().await?;
    let sources = state.list_sources().await?;

    if sources.is_empty() {
        if output.json {
            emit_json(&serde_json::json!({
                "success": true,
                "sources": []
            }))?;
        } else {
            println!("No sources configured.");
            println!("\nAdd a source with:");
            println!("  skillctrl source add <name> --repo <url>");
        }
        return Ok(());
    }

    if output.json {
        let payload: Vec<SourceJsonView> = sources
            .iter()
            .map(|source| SourceJsonView {
                name: source.name.clone(),
                repo_url: source.repo_url.clone(),
                branch: source.branch.clone(),
                auth: format_source_auth(source),
                last_commit: source.last_commit.clone(),
                updated_at: source.updated_at.clone(),
            })
            .collect();
        emit_json(&serde_json::json!({
            "success": true,
            "sources": payload
        }))?;
    } else {
        println!("Configured sources:");
        println!();
        for source in &sources {
            println!("  {}", source.name);
            println!("    URL: {}", source.repo_url);
            println!("    Branch: {}", source.branch);
            println!("    Auth: {}", format_source_auth(source));
            if let Some(commit) = &source.last_commit {
                println!("    Last commit: {}", commit);
            }
            println!();
        }
    }

    Ok(())
}

async fn source_remove(name: String, output: OutputConfig) -> Result<()> {
    if !output.json && !output.quiet {
        println!("Removing source '{}'...", name);
    }

    let state = skillctrl_state::StateManager::open_default().await?;
    let source = state
        .get_source(&name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("source '{}' not found", name))?;

    state.remove_source(&name).await?;

    if source.cache_path.exists() {
        if let Err(err) = fs::remove_dir_all(&source.cache_path) {
            if !output.json {
                eprintln!(
                    "warning: source '{}' was removed from state, but failed to delete cache {}: {}",
                    name,
                    source.cache_path.display(),
                    err
                );
            }
        }
    }

    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "action": "source_remove",
            "source": {
                "name": name
            }
        }))?;
    } else {
        println!("✓ Source '{}' removed", name);
    }
    Ok(())
}

async fn source_update(
    name: String,
    ssh_key: Option<PathBuf>,
    access_token: Option<String>,
    output: OutputConfig,
) -> Result<()> {
    let result = perform_source_update(name, ssh_key, access_token, output).await?;
    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "action": "source_update",
            "source": result
        }))?;
    } else {
        println!("✓ Source '{}' updated successfully", result.name);
    }
    Ok(())
}

async fn perform_source_update(
    name: String,
    ssh_key: Option<PathBuf>,
    access_token: Option<String>,
    output: OutputConfig,
) -> Result<SourceUpdateView> {
    if !output.json && !output.quiet {
        println!("Updating source '{}'...", name);
    }

    let state = skillctrl_state::StateManager::open_default().await?;
    let source_entry = state
        .get_source(&name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("source '{}' not found", name))?;

    let cache_dir = source_entry
        .cache_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid cache path for source '{}'", source_entry.name))?
        .to_path_buf();
    let source = build_git_source(
        source_entry.name.clone(),
        source_entry.repo_url.clone(),
        source_entry.branch.clone(),
        cache_dir.clone(),
        ssh_key.or(source_entry.ssh_key_path.clone()),
        access_token.or(source_entry.access_token.clone()),
    );
    validate_source_auth_args(
        &source.repo_url,
        source.ssh_key_path.as_ref(),
        source.access_token.as_deref(),
    )?;

    let spinner = create_spinner("Fetching updates...".to_string(), output);
    let git_manager = skillctrl_git::GitManager::new(cache_dir);
    let path = git_manager.fetch(&source).await?;
    let catalog = SourceCatalog::load_from_dir(&path)
        .with_context(|| format!("failed to load source catalog from {}", path.display()))?;
    let current_commit = git_manager.current_commit(&source).await.ok();
    spinner.finish_with_message(format!(
        "Updates fetched ({} bundles available)",
        catalog.bundles().len()
    ));

    state.register_source(&source).await?;
    state
        .update_source_sync_status(&name, current_commit.as_deref())
        .await?;

    Ok(SourceUpdateView {
        name,
        repo_url: source.repo_url.clone(),
        branch: source.branch.clone(),
        auth: auth_label_from_args(
            source.ssh_key_path.as_ref(),
            source.access_token.as_deref(),
            &source.repo_url,
        ),
        last_commit: current_commit,
        bundles: catalog.bundles().len(),
    })
}

async fn handle_list(
    source: Option<String>,
    target: Option<String>,
    search: Option<String>,
    output: OutputConfig,
) -> Result<()> {
    let target = match target {
        Some(target) => Some(parse_endpoint(&target)?),
        None => None,
    };
    let query = search.map(|value| value.to_lowercase());
    let sources = load_source_catalogs(source.as_deref()).await?;

    if sources.is_empty() {
        if output.json {
            emit_json(&serde_json::json!({
                "success": true,
                "assets": []
            }))?;
        } else {
            println!("No sources configured.");
            println!("\nAdd a source with:");
            println!("  skillctrl source add <name> --repo <url>");
        }
        return Ok(());
    }

    let mut bundles: Vec<AssetListRow> = Vec::new();
    for source in sources {
        for bundle in source.catalog.bundles() {
            if let Some(target) = &target {
                if !bundle_supports_target(bundle, target) {
                    continue;
                }
            }

            if let Some(query) = &query {
                if !bundle_matches_query(bundle, query) {
                    continue;
                }
            }

            bundles.push(AssetListRow {
                id: bundle.entry.id.clone(),
                name: bundle.manifest.name.clone(),
                source: source.entry.name.clone(),
                version: bundle.manifest.version.to_string(),
                asset_types: bundle_asset_types(bundle),
                targets: bundle_effective_targets(bundle)
                    .iter()
                    .map(|target| target.to_string())
                    .collect(),
                summary: bundle.entry.summary.clone(),
            });
        }
    }

    bundles.sort_by(|left, right| left.id.cmp(&right.id).then(left.source.cmp(&right.source)));

    if bundles.is_empty() {
        if output.json {
            emit_json(&serde_json::json!({
                "success": true,
                "assets": []
            }))?;
        } else {
            println!("Available assets:");
            println!();
            println!("  (no matching assets)");
        }
        return Ok(());
    }

    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "assets": bundles
        }))?;
    } else {
        println!("Available assets:");
        println!();
        print_table(
            &["ID", "Type", "Source", "Version", "Targets", "Summary"],
            &bundles
                .iter()
                .map(|row| {
                    vec![
                        row.id.clone(),
                        row.asset_types.join(", "),
                        row.source.clone(),
                        row.version.clone(),
                        row.targets.join(", "),
                        truncate_text(&row.summary, 72),
                    ]
                })
                .collect::<Vec<_>>(),
        );
    }

    Ok(())
}

async fn handle_show(
    bundle_id: String,
    source: Option<String>,
    output: OutputConfig,
) -> Result<()> {
    let resolved = resolve_bundle(&bundle_id, source.as_deref()).await?;

    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "bundle": {
                "id": resolved.bundle.manifest.id,
                "name": resolved.bundle.manifest.name,
                "source": resolved.source_name,
                "version": resolved.bundle.manifest.version.to_string(),
                "targets": bundle_effective_targets(&resolved.bundle).iter().map(|target| target.to_string()).collect::<Vec<_>>(),
                "description": resolved.bundle.manifest.description,
                "base_path": resolved.bundle.bundle_root.display().to_string(),
                "components": resolved.bundle.manifest.components.iter().map(|component| serde_json::json!({
                    "id": component.id,
                    "kind": component.kind.to_string(),
                    "path": component.path.display().to_string(),
                    "description": component.description,
                })).collect::<Vec<_>>(),
            }
        }))?;
    } else {
        println!("Bundle: {}", resolved.bundle.manifest.id);
        println!("Name: {}", resolved.bundle.manifest.name);
        println!("Source: {}", resolved.source_name);
        println!("Version: {}", resolved.bundle.manifest.version);
        println!(
            "Targets: {}",
            if resolved.bundle.manifest.targets.is_empty() {
                "(none)".to_string()
            } else {
                format_targets(&resolved.bundle.manifest.targets)
            }
        );
        if let Some(description) = &resolved.bundle.manifest.description {
            println!("Description: {}", description);
        }
        println!("Base path: {}", resolved.bundle.bundle_root.display());
        println!();
        println!("Components:");
        for component in &resolved.bundle.manifest.components {
            println!("  - {}: {}", component.kind, component.id);
            println!("    Path: {}", component.path.display());
            if let Some(description) = &component.description {
                println!("    Description: {}", description);
            }
        }
    }

    Ok(())
}

async fn handle_install(
    bundle_id: String,
    source: String,
    target: String,
    scope: String,
    project: Option<PathBuf>,
    dry_run: bool,
    output: OutputConfig,
) -> Result<()> {
    let target = parse_endpoint(&target)?;
    let scope = parse_scope(&scope)?;

    if !output.json {
        println!("Installing bundle '{}'...", bundle_id);
        println!("  Target: {}", target);
        println!("  Scope: {}", scope);
        if let Some(project) = &project {
            println!("  Project: {}", project.display());
        }
    }

    if dry_run {
        if output.json {
            emit_json(&serde_json::json!({
                "success": true,
                "dry_run": true,
                "bundle_id": bundle_id,
                "source": source,
                "target": target.to_string(),
                "scope": scope.to_string(),
                "project_path": project.as_ref().map(|path| path.display().to_string())
            }))?;
        } else {
            println!();
            println!("[DRY RUN] Will plan installation without writing files.");
        }
        return Ok(());
    }

    let spinner = create_spinner("Planning installation...", output);

    // Get adapter
    let adapter = get_adapter(&target)?;

    let resolved = resolve_bundle(&bundle_id, Some(&source)).await?;
    let bundle_root = resolved.bundle.bundle_root.clone();
    let bundle = resolved.bundle.manifest.clone();

    spinner.finish_with_message("Installation planned");

    // Create install context
    let ctx = skillctrl_adapter_core::InstallContext {
        target: target.clone(),
        scope,
        project_path: project.clone(),
        dry_run,
        conflict_strategy: skillctrl_adapter_core::ConflictStrategy::BackupThenWrite,
        metadata: {
            let mut m = std::collections::HashMap::new();
            m.insert(
                "bundle_path".to_string(),
                bundle_root.to_string_lossy().to_string(),
            );
            m
        },
    };

    let spinner = create_spinner("Installing...", output);
    let plan = adapter.plan_install(&bundle, &ctx).await?;
    let result = adapter.apply_install(&plan).await?;
    spinner.finish_with_message("Installation complete");

    let target_label = target.to_string();
    let scope_label = scope.to_string();
    let project_label = project.as_ref().map(|path| path.display().to_string());
    let bundle_version = bundle.version.to_string();

    // Record installation
    let state = skillctrl_state::StateManager::open_default().await?;
    let install_record = skillctrl_state::InstallationRecord {
        bundle_id: bundle.id.clone(),
        bundle_version: bundle.version.clone(),
        source_name: Some(source),
        endpoint: target.clone(),
        scope,
        project_path: project.clone(),
        installed_at: chrono::Utc::now(),
        files_created: result.files_created.clone(),
        backup_path: None,
    };
    state.record_installation(&install_record).await?;

    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "bundle_id": bundle_id,
            "source": resolved.source_name,
            "target": target_label,
            "scope": scope_label,
            "project_path": project_label,
            "version": bundle_version,
            "files_created": result.files_created.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
            "files_modified": result.files_modified.iter().map(|path| path.display().to_string()).collect::<Vec<_>>()
        }))?;
    } else {
        println!("✓ Bundle '{}' installed successfully", bundle_id);
        println!("  Files created: {}", result.files_created.len());
    }

    Ok(())
}

async fn handle_uninstall(
    bundle_id: String,
    target: String,
    scope: String,
    project: Option<PathBuf>,
    dry_run: bool,
    output: OutputConfig,
) -> Result<()> {
    let target = parse_endpoint(&target)?;
    let scope = parse_scope(&scope)?;

    if !output.json {
        println!("Uninstalling bundle '{}'...", bundle_id);
    }

    if dry_run {
        if output.json {
            emit_json(&serde_json::json!({
                "success": true,
                "dry_run": true,
                "bundle_id": bundle_id,
                "target": target.to_string(),
                "scope": scope.to_string(),
                "project_path": project.as_ref().map(|path| path.display().to_string())
            }))?;
        } else {
            println!();
            println!("[DRY RUN] Would remove:");
            println!("  .claude/skills/review-pr/SKILL.md");
            println!("  .claude/rules/review-policy.md");
        }
        return Ok(());
    }

    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "bundle_id": bundle_id,
            "target": target.to_string(),
            "scope": scope.to_string(),
            "project_path": project.as_ref().map(|path| path.display().to_string())
        }))?;
    } else {
        println!("✓ Bundle '{}' uninstalled", bundle_id);
    }
    Ok(())
}

async fn handle_import_command(action: ImportCommands, output: OutputConfig) -> Result<()> {
    match action {
        ImportCommands::Scan { from, path } => import_scan(from, path, output).await,
        ImportCommands::Plan { from, path, id } => import_plan(from, path, id, output).await,
        ImportCommands::Apply { from, path, out } => import_apply(from, path, out, output).await,
    }
}

async fn import_scan(from: String, path: PathBuf, output: OutputConfig) -> Result<()> {
    let endpoint = parse_endpoint(&from)?;

    if !output.json {
        println!("Scanning {} for {} artifacts...", path.display(), endpoint);
    }

    let importer = get_importer(&endpoint)?;

    let spinner = create_spinner("Scanning...", output);
    let req = skillctrl_importer_core::ScanRequest {
        from: endpoint.clone(),
        path: path.clone(),
        depth: 10,
        follow_symlinks: false,
        metadata: std::collections::HashMap::new(),
    };

    let artifacts = importer.scan(&req).await?;
    spinner.finish_with_message("Scan complete");

    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "from": endpoint.to_string(),
            "path": path.display().to_string(),
            "artifacts": artifacts.artifacts.iter().map(|artifact| serde_json::json!({
                "kind": format!("{:?}", artifact.kind),
                "id": artifact.id,
                "path": artifact.path.display().to_string(),
                "description": artifact.description,
            })).collect::<Vec<_>>()
        }))?;
    } else {
        println!();
        println!("Found {} artifacts:", artifacts.artifacts.len());

        for artifact in &artifacts.artifacts {
            println!(
                "  [{:?}] {}",
                artifact.kind,
                artifact.id.as_ref().unwrap_or(&"unknown".to_string())
            );
            println!("    Path: {}", artifact.path.display());
            if let Some(description) = &artifact.description {
                println!("    Description: {}", description);
            }
            println!();
        }
    }

    Ok(())
}

async fn import_plan(
    from: String,
    path: PathBuf,
    id: Option<String>,
    output: OutputConfig,
) -> Result<()> {
    let endpoint = parse_endpoint(&from)?;

    if !output.json {
        println!("Creating import plan from {}...", path.display());
    }

    let importer = get_importer(&endpoint)?;

    let scan_req = skillctrl_importer_core::ScanRequest {
        from: endpoint.clone(),
        path: path.clone(),
        depth: 10,
        follow_symlinks: false,
        metadata: std::collections::HashMap::new(),
    };

    let artifacts = importer.scan(&scan_req).await?;

    let import_req = skillctrl_importer_core::ImportRequest {
        from: endpoint.clone(),
        path: path.clone(),
        bundle_id: id,
        bundle_name: None,
        bundle_description: None,
        preserve_structure: false,
        metadata: std::collections::HashMap::new(),
    };

    let plan = importer.plan_import(&import_req, &artifacts).await?;

    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "from": endpoint.to_string(),
            "path": path.display().to_string(),
            "bundle_id": plan.bundle_id,
            "artifacts": plan.artifacts.iter().map(|artifact| serde_json::json!({
                "kind": artifact.kind.to_string(),
                "path": artifact.path.display().to_string(),
                "id": artifact.id,
                "supported": artifact.supported
            })).collect::<Vec<_>>()
        }))?;
    } else {
        println!("Import plan created:");
        println!("  Bundle ID: {}", plan.bundle_id);
        println!("  Artifacts: {}", plan.artifacts.len());
    }

    Ok(())
}

async fn import_apply(
    from: String,
    path: PathBuf,
    out: PathBuf,
    output: OutputConfig,
) -> Result<()> {
    let endpoint = parse_endpoint(&from)?;

    if !output.json {
        println!("Importing from {} to {}...", path.display(), out.display());
    }

    let importer = get_importer(&endpoint)?;

    let scan_req = skillctrl_importer_core::ScanRequest {
        from: endpoint.clone(),
        path: path.clone(),
        depth: 10,
        follow_symlinks: false,
        metadata: std::collections::HashMap::new(),
    };

    let artifacts = importer.scan(&scan_req).await?;

    let import_req = skillctrl_importer_core::ImportRequest {
        from: endpoint.clone(),
        path: path.clone(),
        bundle_id: None,
        bundle_name: None,
        bundle_description: None,
        preserve_structure: false,
        metadata: std::collections::HashMap::new(),
    };

    let plan = importer.plan_import(&import_req, &artifacts).await?;

    let apply_req = skillctrl_importer_core::ApplyImportRequest {
        plan,
        out: out.clone(),
        overwrite: false,
        metadata: std::collections::HashMap::new(),
    };

    let spinner = create_spinner("Importing...", output);
    let result = importer.apply_import(&apply_req).await?;
    spinner.finish_with_message("Import complete");

    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "from": endpoint.to_string(),
            "path": path.display().to_string(),
            "output": out.display().to_string(),
            "files_created": result.files_created.iter().map(|path| path.display().to_string()).collect::<Vec<_>>()
        }))?;
    } else {
        println!("✓ Import completed successfully");
        println!("  Output: {}", out.display());
        println!("  Files created: {}", result.files_created.len());
    }

    Ok(())
}

async fn handle_status(
    target: String,
    scope: String,
    project: Option<PathBuf>,
    bundle: Option<String>,
    output: OutputConfig,
) -> Result<()> {
    let target = parse_endpoint(&target)?;
    let scope = parse_scope(&scope)?;

    // Query state
    let state = skillctrl_state::StateManager::open_default().await?;
    let installations = state
        .query_installations(
            bundle.as_deref(),
            Some(&target),
            Some(scope),
            project.as_deref(),
        )
        .await?;

    if installations.is_empty() {
        if output.json {
            emit_json(&serde_json::json!({
                "success": true,
                "target": target.to_string(),
                "scope": scope.to_string(),
                "project_path": project.as_ref().map(|path| path.display().to_string()),
                "installations": []
            }))?;
        } else {
            println!("No installations found.");
        }
        return Ok(());
    }

    let views: Vec<InstallRecordView> = installations
        .iter()
        .map(|install| InstallRecordView {
            bundle_id: install.bundle_id.clone(),
            version: install.bundle_version.to_string(),
            source_name: install.source_name.clone(),
            endpoint: install.endpoint.to_string(),
            scope: install.scope.to_string(),
            project_path: install
                .project_path
                .as_ref()
                .map(|path| path.display().to_string()),
            installed_at: install.installed_at.to_rfc3339(),
            files_created: install
                .files_created
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
        })
        .collect();

    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "target": target.to_string(),
            "scope": scope.to_string(),
            "project_path": project.as_ref().map(|path| path.display().to_string()),
            "installations": views
        }))?;
    } else {
        println!("Status for {}:", target);
        println!("  Scope: {}", scope);
        if let Some(project) = &project {
            println!("  Project: {}", project.display());
        }
        println!();
        println!("Installed bundles:");
        print_table(
            &["Bundle", "Version", "Source", "Installed At", "Files"],
            &views
                .iter()
                .map(|view| {
                    vec![
                        view.bundle_id.clone(),
                        view.version.clone(),
                        view.source_name.clone().unwrap_or_else(|| "-".to_string()),
                        view.installed_at.clone(),
                        view.files_created.len().to_string(),
                    ]
                })
                .collect::<Vec<_>>(),
        );
    }

    Ok(())
}

async fn handle_update(source: Option<String>, output: OutputConfig) -> Result<()> {
    if let Some(name) = source {
        source_update(name, None, None, output).await
    } else {
        // Update all sources
        let state = skillctrl_state::StateManager::open_default().await?;
        let sources = state.list_sources().await?;

        if sources.is_empty() {
            if output.json {
                emit_json(&serde_json::json!({
                    "success": true,
                    "updated_sources": []
                }))?;
            } else {
                println!("No sources to update.");
            }
            return Ok(());
        }

        let mut updated = Vec::new();
        for source in &sources {
            let update = perform_source_update(
                source.name.clone(),
                None,
                None,
                OutputConfig {
                    json: false,
                    quiet: output.quiet || output.json,
                },
            )
            .await?;
            updated.push(update);
        }

        if output.json {
            emit_json(&serde_json::json!({
                "success": true,
                "updated_sources": updated
            }))?;
        }

        Ok(())
    }
}

async fn handle_export(
    bundle_id: String,
    source: String,
    target: String,
    out: PathBuf,
    format: String,
    output: OutputConfig,
) -> Result<()> {
    let endpoint = parse_endpoint(&target)?;
    if !output.json {
        println!("Exporting bundle '{}' to {} format...", bundle_id, format);
        println!("  Target: {}", endpoint);
        println!("  Output: {}", out.display());
    }

    let resolved = resolve_bundle(&bundle_id, Some(&source)).await?;
    let bundle_root = resolved.bundle.bundle_root.clone();
    let bundle = resolved.bundle.manifest.clone();

    fs::create_dir_all(&out)?;

    for component in &bundle.components {
        let src = bundle_root.join(&component.path);
        if src.is_file() {
            let dest = out.join(&component.path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src, &dest)?;
            continue;
        }

        if src.is_dir() {
            for entry in walkdir::WalkDir::new(&src) {
                let entry = entry.map_err(|e| anyhow::anyhow!("walk error: {}", e))?;
                let entry_path = entry.path();
                if !entry_path.is_file() {
                    continue;
                }

                let rel = entry_path
                    .strip_prefix(&bundle_root)
                    .map_err(|e| anyhow::anyhow!("path strip error: {}", e))?;
                let dest = out.join(rel);
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(entry_path, &dest)?;
            }
        }
    }

    let bundle_manifest = out.join("bundle.yaml");
    let manifest_content = serde_yaml::to_string(&bundle)
        .map_err(|e| anyhow::anyhow!("failed to serialize bundle manifest: {}", e))?;
    fs::write(&bundle_manifest, manifest_content)?;

    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "bundle_id": bundle_id,
            "source": source,
            "target": endpoint.to_string(),
            "format": format,
            "output": out.display().to_string()
        }))?;
    } else {
        println!("✓ Bundle exported successfully to {}", out.display());
    }

    Ok(())
}

fn handle_completion(shell: Shell, output: OutputConfig) -> Result<()> {
    let script = render_completion_script(shell)?;
    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "shell": shell.to_string(),
            "script": script,
        }))?;
    } else {
        print!("{}", script);
    }
    Ok(())
}

// Helper functions

#[derive(Clone)]
struct LoadedSource {
    entry: skillctrl_state::SourceEntry,
    catalog: SourceCatalog,
}

#[derive(Clone)]
struct ResolvedBundle {
    source_name: String,
    bundle: SourceBundle,
}

async fn load_source_catalogs(source_name: Option<&str>) -> Result<Vec<LoadedSource>> {
    let state = skillctrl_state::StateManager::open_default().await?;
    let sources = state.list_sources().await?;

    if let Some(source_name) = source_name {
        if !sources.iter().any(|source| source.name == source_name) {
            return Err(anyhow::anyhow!("source '{}' not found", source_name));
        }
    }

    let mut loaded = Vec::new();
    for source in sources {
        if source_name.is_some() && source_name != Some(source.name.as_str()) {
            continue;
        }

        let catalog = SourceCatalog::load_from_dir(&source.cache_path).with_context(|| {
            format!(
                "failed to load catalog for source '{}' from {}",
                source.name,
                source.cache_path.display()
            )
        })?;

        loaded.push(LoadedSource {
            entry: source,
            catalog,
        });
    }

    Ok(loaded)
}

async fn resolve_bundle(bundle_id: &str, source_name: Option<&str>) -> Result<ResolvedBundle> {
    let sources = load_source_catalogs(source_name).await?;
    let mut matches = Vec::new();

    for source in sources {
        if let Some(bundle) = source.catalog.find_bundle(bundle_id) {
            matches.push(ResolvedBundle {
                source_name: source.entry.name.clone(),
                bundle: bundle.clone(),
            });
        }
    }

    match matches.len() {
        0 => Err(anyhow::anyhow!("bundle '{}' not found", bundle_id)),
        1 => Ok(matches.remove(0)),
        _ => Err(anyhow::anyhow!(
            "bundle '{}' exists in multiple sources; rerun with --source",
            bundle_id
        )),
    }
}

fn bundle_matches_query(bundle: &SourceBundle, query: &str) -> bool {
    let description = bundle.manifest.description.as_deref().unwrap_or_default();
    bundle.entry.id.to_lowercase().contains(query)
        || bundle.entry.summary.to_lowercase().contains(query)
        || bundle.manifest.name.to_lowercase().contains(query)
        || description.to_lowercase().contains(query)
}

fn bundle_supports_target(bundle: &SourceBundle, target: &Endpoint) -> bool {
    let targets = if bundle.manifest.targets.is_empty() {
        &bundle.entry.targets
    } else {
        &bundle.manifest.targets
    };

    targets.is_empty() || targets.iter().any(|candidate| candidate == target)
}

fn format_targets(targets: &[Endpoint]) -> String {
    targets
        .iter()
        .map(|target| target.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_endpoint(s: &str) -> Result<Endpoint> {
    Endpoint::from_str(s)
}

fn parse_scope(s: &str) -> Result<Scope> {
    let scope = match s.to_lowercase().as_str() {
        "project" | "p" => Scope::Project,
        "user" | "u" | "global" | "g" => Scope::User,
        _ => return Err(anyhow::anyhow!("invalid scope: {}", s)),
    };
    Ok(scope)
}

fn build_git_source(
    name: String,
    repo: String,
    branch: String,
    cache_dir: PathBuf,
    ssh_key: Option<PathBuf>,
    access_token: Option<String>,
) -> skillctrl_git::GitSource {
    let source = skillctrl_git::GitSource::new(name, repo, branch, cache_dir);

    match (ssh_key, access_token) {
        (Some(key_path), None) => source.with_ssh_auth(key_path, None),
        (None, Some(token)) => source.with_https_auth(token),
        (None, None) => source,
        (Some(_), Some(_)) => unreachable!("clap prevents conflicting auth arguments"),
    }
}

fn format_source_auth(source: &skillctrl_state::SourceEntry) -> String {
    match repo_transport(&source.repo_url) {
        RepoTransport::Local => "local".to_string(),
        RepoTransport::Ssh => match &source.ssh_key_path {
            Some(path) => format!("ssh (key: {})", path.display()),
            None => "ssh (agent/default)".to_string(),
        },
        RepoTransport::Https => {
            if source.access_token.is_some() {
                "https (token configured)".to_string()
            } else {
                "https (anonymous/default)".to_string()
            }
        }
    }
}

fn validate_source_auth_args(
    repo: &str,
    ssh_key: Option<&PathBuf>,
    access_token: Option<&str>,
) -> Result<()> {
    if let Some(token) = access_token {
        if token.trim().is_empty() {
            return Err(anyhow::anyhow!("--access-token cannot be empty"));
        }
    }

    match repo_transport(repo) {
        RepoTransport::Local => {
            if ssh_key.is_some() || access_token.is_some() {
                return Err(anyhow::anyhow!(
                    "local repositories do not accept --ssh-key or --access-token"
                ));
            }
        }
        RepoTransport::Ssh => {
            if access_token.is_some() {
                return Err(anyhow::anyhow!(
                    "--access-token can only be used with HTTPS repository URLs"
                ));
            }
        }
        RepoTransport::Https => {
            if ssh_key.is_some() {
                return Err(anyhow::anyhow!(
                    "--ssh-key can only be used with SSH repository URLs"
                ));
            }
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RepoTransport {
    Local,
    Ssh,
    Https,
}

fn repo_transport(repo: &str) -> RepoTransport {
    if repo.starts_with("http://") || repo.starts_with("https://") {
        return RepoTransport::Https;
    }

    if repo.starts_with("ssh://")
        || (repo.contains('@') && repo.contains(':') && !repo.contains("://"))
    {
        return RepoTransport::Ssh;
    }

    RepoTransport::Local
}

async fn handle_verify(
    bundle_id: String,
    source: Option<String>,
    target: String,
    scope: String,
    project: Option<PathBuf>,
    output: OutputConfig,
) -> Result<()> {
    let target = parse_endpoint(&target)?;
    let scope = parse_scope(&scope)?;
    let resolved = resolve_bundle(&bundle_id, source.as_deref()).await?;
    let state = skillctrl_state::StateManager::open_default().await?;
    let installation = state
        .query_installations(
            Some(&bundle_id),
            Some(&target),
            Some(scope),
            project.as_deref(),
        )
        .await?
        .into_iter()
        .next();

    let verification = verify_bundle_installation(
        &resolved,
        &target,
        scope,
        project.as_deref(),
        installation.as_ref(),
    )?;

    if output.json {
        emit_json(&serde_json::json!({
            "success": true,
            "verification": verification
        }))?;
    } else {
        println!("Verification for {}:", verification.bundle_id);
        println!("  Source: {}", verification.source);
        println!("  Target: {}", verification.target);
        println!("  Scope: {}", verification.scope);
        if let Some(project_path) = &verification.project_path {
            println!("  Project: {}", project_path);
        }
        println!(
            "  Installed: {}",
            if verification.installed { "yes" } else { "no" }
        );
        println!(
            "  Latest Version: {} ({})",
            verification.latest_version,
            if verification.is_latest_version {
                "installed"
            } else {
                "outdated or unknown"
            }
        );
        println!(
            "  Content Matches Source: {}",
            if verification.local_matches_source {
                "yes"
            } else {
                "no"
            }
        );
        println!();
        print_table(
            &["Component", "Kind", "Installed", "Content", "Detail"],
            &verification
                .components
                .iter()
                .map(|component| {
                    vec![
                        component.id.clone(),
                        component.kind.clone(),
                        yes_no(component.installed).to_string(),
                        yes_no(component.content_matches).to_string(),
                        component.detail.clone(),
                    ]
                })
                .collect::<Vec<_>>(),
        );
    }

    Ok(())
}

fn verify_bundle_installation(
    resolved: &ResolvedBundle,
    target: &Endpoint,
    scope: Scope,
    project_path: Option<&std::path::Path>,
    installation: Option<&skillctrl_state::InstallationRecord>,
) -> Result<VerifyBundleView> {
    let mut components = Vec::new();
    let mut files_checked = 0usize;
    let mut files_matching = 0usize;

    for component in &resolved.bundle.manifest.components {
        let view = verify_component(
            target,
            scope,
            project_path,
            &resolved.bundle.bundle_root,
            component,
        )?;
        files_checked += view.files.len();
        files_matching += view
            .files
            .iter()
            .filter(|file| file.matches_expected)
            .count();
        components.push(view);
    }

    let installed =
        installation.is_some() || components.iter().any(|component| component.installed);
    let local_matches_source =
        !components.is_empty() && components.iter().all(|component| component.content_matches);
    let installed_version = installation.map(|install| install.bundle_version.to_string());
    let latest_version = resolved.bundle.manifest.version.to_string();
    let is_latest_version = installed_version
        .as_deref()
        .map(|version| version == latest_version)
        .unwrap_or(false);

    Ok(VerifyBundleView {
        bundle_id: resolved.bundle.manifest.id.clone(),
        source: resolved.source_name.clone(),
        target: target.to_string(),
        scope: scope.to_string(),
        project_path: project_path.map(|path| path.display().to_string()),
        installed,
        installed_version,
        latest_version,
        is_latest_version,
        local_matches_source,
        installation_record_found: installation.is_some(),
        files_checked,
        files_matching,
        components,
    })
}

fn verify_component(
    target: &Endpoint,
    scope: Scope,
    project_path: Option<&std::path::Path>,
    bundle_root: &std::path::Path,
    component: &skillctrl_core::ComponentRef,
) -> Result<VerifyComponentView> {
    let source_path = bundle_root.join(&component.path);

    match target {
        Endpoint::Known(KnownEndpoint::ClaudeCode) => {
            verify_claude_component(scope, project_path, component, &source_path)
        }
        Endpoint::Known(KnownEndpoint::Codex) => {
            verify_codex_component(scope, project_path, component, &source_path)
        }
        Endpoint::Known(KnownEndpoint::Cursor) => {
            verify_cursor_component(scope, project_path, component, &source_path)
        }
        _ => Ok(VerifyComponentView {
            id: component.id.clone(),
            kind: component.kind.to_string(),
            installed: false,
            content_matches: false,
            detail: format!("verification not implemented for target {}", target),
            files: Vec::new(),
        }),
    }
}

fn verify_claude_component(
    scope: Scope,
    project_path: Option<&std::path::Path>,
    component: &skillctrl_core::ComponentRef,
    source_path: &std::path::Path,
) -> Result<VerifyComponentView> {
    let claude_dir = claude_dir(scope, project_path)?;

    match component.kind {
        ComponentKind::Skill => verify_text_component(
            component,
            source_path,
            &claude_dir
                .join("skills")
                .join(&component.id)
                .join("SKILL.md"),
        ),
        ComponentKind::Command => verify_text_component(
            component,
            source_path,
            &claude_dir
                .join("commands")
                .join(format!("{}.md", component.id)),
        ),
        ComponentKind::Rule => verify_text_component(
            component,
            source_path,
            &claude_dir
                .join("rules")
                .join(format!("{}.md", component.id)),
        ),
        ComponentKind::Resource => verify_text_component(
            component,
            source_path,
            &claude_dir.join("resources").join(&component.id),
        ),
        ComponentKind::Agent => verify_directory_component(
            component,
            source_path,
            &claude_dir.join("agents").join(&component.id),
            Some("agent.md"),
        ),
        ComponentKind::McpServer => {
            let target_path = match scope {
                Scope::Project => project_path
                    .ok_or_else(|| anyhow::anyhow!("project path required for project scope"))?
                    .join(".mcp.json"),
                Scope::User => dirs::home_dir()
                    .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?
                    .join(".claude.json"),
            };
            verify_json_subtree_component(
                component,
                source_path,
                &target_path,
                |source_value, installed_value| {
                    installed_value
                        .get("mcpServers")
                        .and_then(|servers| servers.get(&component.id))
                        .map(|value| value == source_value)
                        .unwrap_or(false)
                },
                format!("MCP entry {}", component.id),
            )
        }
        ComponentKind::Hook => verify_json_subtree_component(
            component,
            source_path,
            &claude_dir.join("settings.json"),
            |source_value, installed_value| {
                installed_value.get("hooks") == source_value.get("hooks")
            },
            "hooks config".to_string(),
        ),
        _ => unsupported_verify_component(component, "Claude"),
    }
}

fn verify_codex_component(
    scope: Scope,
    project_path: Option<&std::path::Path>,
    component: &skillctrl_core::ComponentRef,
    source_path: &std::path::Path,
) -> Result<VerifyComponentView> {
    let codex_dir = codex_dir(scope, project_path)?;

    match component.kind {
        ComponentKind::Skill => verify_text_component(
            component,
            source_path,
            &codex_dir
                .join("skills")
                .join(&component.id)
                .join("SKILL.md"),
        ),
        ComponentKind::Resource => verify_text_component(
            component,
            source_path,
            &codex_dir.join("assets").join(&component.id),
        ),
        ComponentKind::Rule => {
            let agents_path = codex_dir.join("AGENTS.md");
            let source_content = fs::read_to_string(source_path)?;
            let expected_block = format!("# {}\n\n{}", component.id, source_content);
            let installed_content = fs::read_to_string(&agents_path).unwrap_or_default();
            let matches_expected = installed_content.contains(&expected_block);
            Ok(VerifyComponentView {
                id: component.id.clone(),
                kind: component.kind.to_string(),
                installed: agents_path.exists(),
                content_matches: matches_expected,
                detail: if matches_expected {
                    "rule block present in AGENTS.md".to_string()
                } else {
                    "rule block missing from AGENTS.md".to_string()
                },
                files: vec![VerifyFileView {
                    path: agents_path.display().to_string(),
                    exists: agents_path.exists(),
                    matches_expected,
                    detail: "contains expected rule block".to_string(),
                }],
            })
        }
        ComponentKind::McpServer => {
            let config_path = codex_dir.join("config.toml");
            let source_value: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(source_path)?)?;
            let expected_value = toml::Value::try_from(source_value)
                .map_err(|e| anyhow::anyhow!("failed to convert MCP config to TOML: {}", e))?;

            let installed_value = if config_path.exists() {
                let raw = fs::read_to_string(&config_path)?;
                raw.parse::<toml::Value>().map_err(|e| {
                    anyhow::anyhow!("failed to parse {}: {}", config_path.display(), e)
                })?
            } else {
                toml::Value::Table(toml::map::Map::new())
            };

            let matches_expected = installed_value
                .get("mcpServers")
                .and_then(|servers| servers.get(&component.id))
                .map(|value| value == &expected_value)
                .unwrap_or(false);

            Ok(VerifyComponentView {
                id: component.id.clone(),
                kind: component.kind.to_string(),
                installed: config_path.exists(),
                content_matches: matches_expected,
                detail: if matches_expected {
                    "MCP entry matches config.toml".to_string()
                } else {
                    "MCP entry missing or different in config.toml".to_string()
                },
                files: vec![VerifyFileView {
                    path: config_path.display().to_string(),
                    exists: config_path.exists(),
                    matches_expected,
                    detail: format!("mcpServers.{}", component.id),
                }],
            })
        }
        _ => unsupported_verify_component(component, "Codex"),
    }
}

fn verify_cursor_component(
    scope: Scope,
    project_path: Option<&std::path::Path>,
    component: &skillctrl_core::ComponentRef,
    source_path: &std::path::Path,
) -> Result<VerifyComponentView> {
    let cursor_dir = cursor_dir(scope, project_path)?;

    match component.kind {
        ComponentKind::Rule => {
            let expected = cursor_rule_mdc(component, &fs::read_to_string(source_path)?)?;
            verify_text_content_against_path(
                component,
                &expected,
                &cursor_dir
                    .join("rules")
                    .join(format!("{}.mdc", component.id)),
            )
        }
        ComponentKind::Skill => {
            let expected = cursor_skill_mdc(component, &fs::read_to_string(source_path)?)?;
            verify_text_content_against_path(
                component,
                &expected,
                &cursor_dir
                    .join("rules")
                    .join(format!("{}.mdc", component.id)),
            )
        }
        ComponentKind::Resource => verify_text_component(
            component,
            source_path,
            &cursor_dir.join("resources").join(&component.id),
        ),
        _ => unsupported_verify_component(component, "Cursor"),
    }
}

fn verify_text_component(
    component: &skillctrl_core::ComponentRef,
    source_path: &std::path::Path,
    target_path: &std::path::Path,
) -> Result<VerifyComponentView> {
    let expected = fs::read_to_string(source_path)?;
    verify_text_content_against_path(component, &expected, target_path)
}

fn verify_text_content_against_path(
    component: &skillctrl_core::ComponentRef,
    expected: &str,
    target_path: &std::path::Path,
) -> Result<VerifyComponentView> {
    let installed = target_path.exists();
    let actual = if installed {
        fs::read_to_string(target_path).unwrap_or_default()
    } else {
        String::new()
    };
    let matches_expected = installed && actual == expected;

    Ok(VerifyComponentView {
        id: component.id.clone(),
        kind: component.kind.to_string(),
        installed,
        content_matches: matches_expected,
        detail: if matches_expected {
            "file content matches source".to_string()
        } else if installed {
            "file content differs from source".to_string()
        } else {
            "target file missing".to_string()
        },
        files: vec![VerifyFileView {
            path: target_path.display().to_string(),
            exists: installed,
            matches_expected,
            detail: "exact content match".to_string(),
        }],
    })
}

fn verify_directory_component(
    component: &skillctrl_core::ComponentRef,
    source_path: &std::path::Path,
    target_root: &std::path::Path,
    single_file_name: Option<&str>,
) -> Result<VerifyComponentView> {
    let mut files = Vec::new();

    if source_path.is_dir() {
        for entry in walkdir::WalkDir::new(source_path)
            .min_depth(1)
            .max_depth(10)
        {
            let entry = entry.map_err(|e| anyhow::anyhow!("walk error: {}", e))?;
            if !entry.path().is_file() {
                continue;
            }
            let rel_path = entry.path().strip_prefix(source_path)?;
            let expected = fs::read_to_string(entry.path())?;
            let target_path = target_root.join(rel_path);
            let exists = target_path.exists();
            let matches_expected =
                exists && fs::read_to_string(&target_path).unwrap_or_default() == expected;
            files.push(VerifyFileView {
                path: target_path.display().to_string(),
                exists,
                matches_expected,
                detail: rel_path.display().to_string(),
            });
        }
    } else {
        let target_path = target_root.join(single_file_name.unwrap_or("agent.md"));
        let expected = fs::read_to_string(source_path)?;
        let exists = target_path.exists();
        let matches_expected =
            exists && fs::read_to_string(&target_path).unwrap_or_default() == expected;
        files.push(VerifyFileView {
            path: target_path.display().to_string(),
            exists,
            matches_expected,
            detail: "single-file component".to_string(),
        });
    }

    let installed = !files.is_empty() && files.iter().all(|file| file.exists);
    let content_matches = !files.is_empty() && files.iter().all(|file| file.matches_expected);
    Ok(VerifyComponentView {
        id: component.id.clone(),
        kind: component.kind.to_string(),
        installed,
        content_matches,
        detail: if content_matches {
            "all files match source".to_string()
        } else {
            "one or more files are missing or different".to_string()
        },
        files,
    })
}

fn verify_json_subtree_component(
    component: &skillctrl_core::ComponentRef,
    source_path: &std::path::Path,
    target_path: &std::path::Path,
    predicate: impl Fn(&serde_json::Value, &serde_json::Value) -> bool,
    detail_label: String,
) -> Result<VerifyComponentView> {
    let source_value: serde_json::Value = serde_json::from_str(&fs::read_to_string(source_path)?)?;
    let installed = target_path.exists();
    let installed_value = if installed {
        serde_json::from_str::<serde_json::Value>(&fs::read_to_string(target_path)?)
            .unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::Value::Null
    };
    let matches_expected = installed && predicate(&source_value, &installed_value);

    Ok(VerifyComponentView {
        id: component.id.clone(),
        kind: component.kind.to_string(),
        installed,
        content_matches: matches_expected,
        detail: if matches_expected {
            format!("{} matches source", detail_label)
        } else {
            format!("{} missing or different", detail_label)
        },
        files: vec![VerifyFileView {
            path: target_path.display().to_string(),
            exists: installed,
            matches_expected,
            detail: detail_label,
        }],
    })
}

fn unsupported_verify_component(
    component: &skillctrl_core::ComponentRef,
    endpoint_name: &str,
) -> Result<VerifyComponentView> {
    Ok(VerifyComponentView {
        id: component.id.clone(),
        kind: component.kind.to_string(),
        installed: false,
        content_matches: false,
        detail: format!("{} does not support {}", endpoint_name, component.kind),
        files: Vec::new(),
    })
}

fn claude_dir(scope: Scope, project_path: Option<&std::path::Path>) -> Result<PathBuf> {
    Ok(match scope {
        Scope::Project => project_path
            .ok_or_else(|| anyhow::anyhow!("project path required for project scope"))?
            .join(".claude"),
        Scope::User => dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?
            .join(".claude"),
    })
}

fn codex_dir(scope: Scope, project_path: Option<&std::path::Path>) -> Result<PathBuf> {
    Ok(match scope {
        Scope::Project => project_path
            .ok_or_else(|| anyhow::anyhow!("project path required for project scope"))?
            .join(".codex"),
        Scope::User => dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("could not determine config directory"))?
            .join("codex"),
    })
}

fn cursor_dir(scope: Scope, project_path: Option<&std::path::Path>) -> Result<PathBuf> {
    Ok(match scope {
        Scope::Project => project_path
            .ok_or_else(|| anyhow::anyhow!("project path required for project scope"))?
            .join(".cursor"),
        Scope::User => dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("could not determine config directory"))?
            .join("cursor"),
    })
}

fn cursor_rule_mdc(component: &skillctrl_core::ComponentRef, content: &str) -> Result<String> {
    let mut mdc = String::new();
    mdc.push_str("---\n");
    mdc.push_str(&format!("id: {}\n", component.id));
    if let Some(name) = &component.display_name {
        mdc.push_str(&format!("name: {}\n", name));
    }
    if let Some(description) = &component.description {
        mdc.push_str(&format!("description: {}\n", description));
    }
    mdc.push_str("---\n\n");
    mdc.push_str(content);
    Ok(mdc)
}

fn cursor_skill_mdc(component: &skillctrl_core::ComponentRef, content: &str) -> Result<String> {
    let mut mdc = String::new();
    mdc.push_str("---\n");
    mdc.push_str(&format!("id: {}\n", component.id));
    mdc.push_str("kind: skill\n");
    if let Some(description) = &component.description {
        mdc.push_str(&format!("description: {}\n", description));
    }
    mdc.push_str("---\n\n");
    mdc.push_str(content);
    Ok(mdc)
}

fn emit_json(value: &serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn render_completion_script(shell: Shell) -> Result<String> {
    let mut cmd = Cli::command();
    let mut buffer = Vec::new();
    generate(shell, &mut cmd, "skillctrl", &mut buffer);
    String::from_utf8(buffer)
        .map_err(|e| anyhow::anyhow!("failed to render completion script as UTF-8: {}", e))
}

fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(index) {
                *width = (*width).max(cell.len());
            }
        }
    }

    let header_line = headers
        .iter()
        .enumerate()
        .map(|(index, header)| format!("{:<width$}", header, width = widths[index]))
        .collect::<Vec<_>>()
        .join("  ");
    println!("{}", header_line);
    println!(
        "{}",
        widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<_>>()
            .join("  ")
    );

    for row in rows {
        println!(
            "{}",
            row.iter()
                .enumerate()
                .map(|(index, cell)| format!("{:<width$}", cell, width = widths[index]))
                .collect::<Vec<_>>()
                .join("  ")
        );
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn bundle_asset_types(bundle: &SourceBundle) -> Vec<String> {
    let mut kinds = BTreeSet::new();
    for component in &bundle.manifest.components {
        kinds.insert(component_kind_label(&component.kind).to_string());
    }
    kinds.into_iter().collect()
}

fn component_kind_label(kind: &ComponentKind) -> &'static str {
    match kind {
        ComponentKind::Skill => "skill",
        ComponentKind::Rule => "rule",
        ComponentKind::Command => "command",
        ComponentKind::McpServer => "mcp",
        ComponentKind::Hook => "hook",
        ComponentKind::Resource => "resource",
        ComponentKind::Agent => "agent",
        _ => "unknown",
    }
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }

    let mut truncated = value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

fn bundle_effective_targets(bundle: &SourceBundle) -> Vec<Endpoint> {
    if bundle.manifest.targets.is_empty() {
        bundle.entry.targets.clone()
    } else {
        bundle.manifest.targets.clone()
    }
}

fn auth_label_from_args(
    ssh_key_path: Option<&PathBuf>,
    access_token: Option<&str>,
    repo_url: &str,
) -> String {
    match repo_transport(repo_url) {
        RepoTransport::Local => "local".to_string(),
        RepoTransport::Ssh => match ssh_key_path {
            Some(path) => format!("ssh (key: {})", path.display()),
            None => "ssh (agent/default)".to_string(),
        },
        RepoTransport::Https => {
            if access_token.is_some() {
                "https (token configured)".to_string()
            } else {
                "https (anonymous/default)".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn component_kind_label_is_human_friendly() {
        assert_eq!(component_kind_label(&ComponentKind::McpServer), "mcp");
        assert_eq!(component_kind_label(&ComponentKind::Resource), "resource");
    }

    #[test]
    fn completion_script_contains_root_command() {
        let script = render_completion_script(Shell::Bash).unwrap();
        assert!(script.contains("skillctrl"));
    }
}

fn get_adapter(
    endpoint: &Endpoint,
) -> Result<std::sync::Arc<dyn skillctrl_adapter_core::InstallAdapter + Send + Sync>> {
    match endpoint {
        Endpoint::Known(KnownEndpoint::ClaudeCode) => Ok(std::sync::Arc::new(
            skillctrl_adapter_claude::ClaudeAdapter::new(),
        )),
        Endpoint::Known(KnownEndpoint::Codex) => Ok(std::sync::Arc::new(
            skillctrl_adapter_codex::CodexAdapter::new(),
        )),
        Endpoint::Known(KnownEndpoint::Cursor) => Ok(std::sync::Arc::new(
            skillctrl_adapter_cursor::CursorAdapter::new(),
        )),
        _ => Err(anyhow::anyhow!("adapter not implemented for: {}", endpoint)),
    }
}

fn get_importer(
    endpoint: &Endpoint,
) -> Result<std::sync::Arc<dyn skillctrl_importer_core::Importer + Send + Sync>> {
    match endpoint {
        Endpoint::Known(KnownEndpoint::ClaudeCode) => Ok(std::sync::Arc::new(
            skillctrl_importer_claude::ClaudeImporter::new(),
        )),
        _ => Err(anyhow::anyhow!(
            "importer not implemented for: {}",
            endpoint
        )),
    }
}

fn create_spinner(msg: impl Into<String>, output: OutputConfig) -> indicatif::ProgressBar {
    if output.json || output.quiet {
        let pb = ProgressBar::hidden();
        pb.set_message(msg.into());
        return pb;
    }

    let style = ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .template("{spinner} {msg}")
        .unwrap();

    let pb = ProgressBar::new_spinner();
    pb.set_style(style);
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb.set_message(msg.into());
    pb
}
