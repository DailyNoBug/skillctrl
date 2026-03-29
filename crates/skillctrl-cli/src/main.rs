//! skillctrl - Unified skills marketplace for Claude Code, Codex, and Cursor.

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use indicatif::{ProgressBar, ProgressStyle};
use skillctrl_catalog::{SourceBundle, SourceCatalog};
use skillctrl_core::{Endpoint, KnownEndpoint, Scope};
use std::fs;
use std::io;
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

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
            eprintln!("\x1b[31merror:\x1b[0m {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_command(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Source { action } => handle_source_command(action).await,
        Commands::List {
            source,
            target,
            search,
        } => handle_list(source, target, search).await,
        Commands::Show { bundle_id, source } => handle_show(bundle_id, source).await,
        Commands::Install {
            bundle_id,
            source,
            target,
            scope,
            project,
            dry_run,
        } => handle_install(bundle_id, source, target, scope, project, dry_run).await,
        Commands::Uninstall {
            bundle_id,
            target,
            scope,
            project,
            dry_run,
        } => handle_uninstall(bundle_id, target, scope, project, dry_run).await,
        Commands::Import { action } => handle_import_command(action).await,
        Commands::Status {
            target,
            scope,
            project,
            bundle,
        } => handle_status(target, scope, project, bundle).await,
        Commands::Update { source } => handle_update(source).await,
        Commands::Export {
            bundle_id,
            source,
            target,
            out,
            format,
        } => handle_export(bundle_id, source, target, out, format).await,
        Commands::Completion { shell } => handle_completion(shell),
    }
}

async fn handle_source_command(action: SourceCommands) -> Result<()> {
    match action {
        SourceCommands::Add {
            name,
            repo,
            branch,
            ssh_key,
            access_token,
        } => source_add(name, repo, branch, ssh_key, access_token).await,
        SourceCommands::List => source_list().await,
        SourceCommands::Remove { name } => source_remove(name).await,
        SourceCommands::Update {
            name,
            ssh_key,
            access_token,
        } => source_update(name, ssh_key, access_token).await,
    }
}

async fn source_add(
    name: String,
    repo: String,
    branch: String,
    ssh_key: Option<PathBuf>,
    access_token: Option<String>,
) -> Result<()> {
    validate_source_auth_args(&repo, ssh_key.as_ref(), access_token.as_deref())?;
    println!("Adding source '{}' from {}", name, repo);

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
    let spinner = create_spinner("Cloning repository...".to_string());
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

    println!("✓ Source '{}' added successfully", name);
    Ok(())
}

async fn source_list() -> Result<()> {
    let state = skillctrl_state::StateManager::open_default().await?;
    let sources = state.list_sources().await?;

    if sources.is_empty() {
        println!("No sources configured.");
        println!("\nAdd a source with:");
        println!("  skillctrl source add <name> --repo <url>");
        return Ok(());
    }

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

    Ok(())
}

async fn source_remove(name: String) -> Result<()> {
    println!("Removing source '{}'...", name);

    let state = skillctrl_state::StateManager::open_default().await?;
    let source = state
        .get_source(&name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("source '{}' not found", name))?;

    state.remove_source(&name).await?;

    if source.cache_path.exists() {
        if let Err(err) = fs::remove_dir_all(&source.cache_path) {
            eprintln!(
                "warning: source '{}' was removed from state, but failed to delete cache {}: {}",
                name,
                source.cache_path.display(),
                err
            );
        }
    }

    println!("✓ Source '{}' removed", name);
    Ok(())
}

async fn source_update(
    name: String,
    ssh_key: Option<PathBuf>,
    access_token: Option<String>,
) -> Result<()> {
    println!("Updating source '{}'...", name);

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

    let spinner = create_spinner("Fetching updates...".to_string());
    let git_manager = skillctrl_git::GitManager::new(cache_dir.clone());
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
    println!("✓ Source '{}' updated successfully", name);
    Ok(())
}

async fn handle_list(
    source: Option<String>,
    target: Option<String>,
    search: Option<String>,
) -> Result<()> {
    let target = match target {
        Some(target) => Some(parse_endpoint(&target)?),
        None => None,
    };
    let query = search.map(|value| value.to_lowercase());
    let sources = load_source_catalogs(source.as_deref()).await?;

    if sources.is_empty() {
        println!("No sources configured.");
        println!("\nAdd a source with:");
        println!("  skillctrl source add <name> --repo <url>");
        return Ok(());
    }

    let mut bundles: Vec<(String, SourceBundle)> = Vec::new();
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

            bundles.push((source.entry.name.clone(), bundle.clone()));
        }
    }

    bundles.sort_by(|left, right| {
        left.1
            .entry
            .id
            .cmp(&right.1.entry.id)
            .then(left.0.cmp(&right.0))
    });

    println!("Available bundles:");
    println!();

    if bundles.is_empty() {
        println!("  (no matching bundles)");
        return Ok(());
    }

    for (source_name, bundle) in bundles {
        println!("  {} [{}]", bundle.entry.id, source_name);
        println!("    {}", bundle.entry.summary);
        if !bundle.manifest.targets.is_empty() {
            println!("    Targets: {}", format_targets(&bundle.manifest.targets));
        }
    }

    Ok(())
}

async fn handle_show(bundle_id: String, source: Option<String>) -> Result<()> {
    let resolved = resolve_bundle(&bundle_id, source.as_deref()).await?;

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

    Ok(())
}

async fn handle_install(
    bundle_id: String,
    source: String,
    target: String,
    scope: String,
    project: Option<PathBuf>,
    dry_run: bool,
) -> Result<()> {
    let target = parse_endpoint(&target)?;
    let scope = parse_scope(&scope)?;

    println!("Installing bundle '{}'...", bundle_id);
    println!("  Target: {}", target);
    println!("  Scope: {}", scope);
    if let Some(project) = &project {
        println!("  Project: {}", project.display());
    }

    if dry_run {
        println!();
        println!("[DRY RUN] Will plan installation without writing files.");
        return Ok(());
    }

    let spinner = create_spinner("Planning installation...");

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

    let spinner = create_spinner("Installing...");
    let plan = adapter.plan_install(&bundle, &ctx).await?;
    let result = adapter.apply_install(&plan).await?;
    spinner.finish_with_message("Installation complete");

    // Record installation
    let state = skillctrl_state::StateManager::open_default().await?;
    let install_record = skillctrl_state::InstallationRecord {
        bundle_id: bundle.id.clone(),
        bundle_version: bundle.version,
        source_name: Some(source),
        endpoint: target,
        scope,
        project_path: project,
        installed_at: chrono::Utc::now(),
        files_created: result.files_created.clone(),
        backup_path: None,
    };
    state.record_installation(&install_record).await?;

    println!("✓ Bundle '{}' installed successfully", bundle_id);
    println!("  Files created: {}", result.files_created.len());

    Ok(())
}

async fn handle_uninstall(
    bundle_id: String,
    target: String,
    scope: String,
    project: Option<PathBuf>,
    dry_run: bool,
) -> Result<()> {
    let target = parse_endpoint(&target)?;
    let scope = parse_scope(&scope)?;

    println!("Uninstalling bundle '{}'...", bundle_id);

    if dry_run {
        println!();
        println!("[DRY RUN] Would remove:");
        println!("  .claude/skills/review-pr/SKILL.md");
        println!("  .claude/rules/review-policy.md");
        return Ok(());
    }

    println!("✓ Bundle '{}' uninstalled", bundle_id);
    Ok(())
}

async fn handle_import_command(action: ImportCommands) -> Result<()> {
    match action {
        ImportCommands::Scan { from, path } => import_scan(from, path).await,
        ImportCommands::Plan { from, path, id } => import_plan(from, path, id).await,
        ImportCommands::Apply { from, path, out } => import_apply(from, path, out).await,
    }
}

async fn import_scan(from: String, path: PathBuf) -> Result<()> {
    let endpoint = parse_endpoint(&from)?;

    println!("Scanning {} for {} artifacts...", path.display(), endpoint);

    let importer = get_importer(&endpoint)?;

    let spinner = create_spinner("Scanning...");
    let req = skillctrl_importer_core::ScanRequest {
        from: endpoint.clone(),
        path: path.clone(),
        depth: 10,
        follow_symlinks: false,
        metadata: std::collections::HashMap::new(),
    };

    let artifacts = importer.scan(&req).await?;
    spinner.finish_with_message("Scan complete");

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

    Ok(())
}

async fn import_plan(from: String, path: PathBuf, id: Option<String>) -> Result<()> {
    let endpoint = parse_endpoint(&from)?;

    println!("Creating import plan from {}...", path.display());

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

    println!("Import plan created:");
    println!("  Bundle ID: {}", plan.bundle_id);
    println!("  Artifacts: {}", plan.artifacts.len());

    Ok(())
}

async fn import_apply(from: String, path: PathBuf, out: PathBuf) -> Result<()> {
    let endpoint = parse_endpoint(&from)?;

    println!("Importing from {} to {}...", path.display(), out.display());

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

    let spinner = create_spinner("Importing...");
    let result = importer.apply_import(&apply_req).await?;
    spinner.finish_with_message("Import complete");

    println!("✓ Import completed successfully");
    println!("  Output: {}", out.display());
    println!("  Files created: {}", result.files_created.len());

    Ok(())
}

async fn handle_status(
    target: String,
    scope: String,
    project: Option<PathBuf>,
    bundle: Option<String>,
) -> Result<()> {
    let target = parse_endpoint(&target)?;
    let scope = parse_scope(&scope)?;

    println!("Status for {}:", target);
    println!("  Scope: {}", scope);
    if let Some(project) = &project {
        println!("  Project: {}", project.display());
    }
    println!();

    // Query state
    let state = skillctrl_state::StateManager::open_default().await?;
    let installations = state
        .query_installations(bundle.as_deref(), Some(&target), Some(scope))
        .await?;

    if installations.is_empty() {
        println!("No installations found.");
        return Ok(());
    }

    println!("Installed bundles:");
    for install in &installations {
        println!("  {} ({})", install.bundle_id, install.bundle_version);
        println!(
            "    Installed: {}",
            install.installed_at.format("%Y-%m-%d %H:%M:%S")
        );
        println!("    Files: {}", install.files_created.len());
        println!();
    }

    Ok(())
}

async fn handle_update(source: Option<String>) -> Result<()> {
    if let Some(name) = source {
        source_update(name, None, None).await
    } else {
        // Update all sources
        let state = skillctrl_state::StateManager::open_default().await?;
        let sources = state.list_sources().await?;

        if sources.is_empty() {
            println!("No sources to update.");
            return Ok(());
        }

        for source in &sources {
            source_update(source.name.clone(), None, None).await?;
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
) -> Result<()> {
    let endpoint = parse_endpoint(&target)?;

    println!("Exporting bundle '{}' to {} format...", bundle_id, format);
    println!("  Target: {}", endpoint);
    println!("  Output: {}", out.display());

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

    println!("✓ Bundle exported successfully to {}", out.display());

    Ok(())
}

fn handle_completion(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "skillctrl", &mut io::stdout());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
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

fn create_spinner(msg: impl Into<String>) -> indicatif::ProgressBar {
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
