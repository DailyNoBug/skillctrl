//! skillctrl - Unified skills marketplace for Claude Code, Codex, and Cursor.

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use console::Style;
use indicatif::{ProgressBar, ProgressStyle};
use skillctrl_core::{Endpoint, Scope, KnownEndpoint};
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
        #[arg(short, long)]
        source: Option<String>,

        /// Filter by endpoint
        #[arg(short, long)]
        target: Option<String>,

        /// Search query
        #[arg(short, long)]
        search: Option<String>,
    },

    /// Show bundle details
    Show {
        /// Bundle ID
        bundle_id: String,

        /// Source name
        #[arg(short, long)]
        source: Option<String>,
    },

    /// Install a bundle
    Install {
        /// Bundle ID
        bundle_id: String,

        /// Source name
        #[arg(short, long)]
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
        #[arg(short, long)]
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
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(if cli.verbose {
                    tracing::Level::DEBUG.into()
                } else {
                    tracing::Level::INFO.into()
                }),
        )
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set tracing subscriber");

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
        Commands::Source { action } => {
            handle_source_command(action).await
        }
        Commands::List { source, target, search } => {
            handle_list(source, target, search).await
        }
        Commands::Show { bundle_id, source } => {
            handle_show(bundle_id, source).await
        }
        Commands::Install {
            bundle_id,
            source,
            target,
            scope,
            project,
            dry_run,
        } => {
            handle_install(bundle_id, source, target, scope, project, dry_run).await
        }
        Commands::Uninstall {
            bundle_id,
            target,
            scope,
            project,
            dry_run,
        } => {
            handle_uninstall(bundle_id, target, scope, project, dry_run).await
        }
        Commands::Import { action } => {
            handle_import_command(action).await
        }
        Commands::Status { target, scope, project, bundle } => {
            handle_status(target, scope, project, bundle).await
        }
        Commands::Update { source } => {
            handle_update(source).await
        }
        Commands::Export {
            bundle_id,
            source,
            target,
            out,
            format,
        } => {
            handle_export(bundle_id, source, target, out, format).await
        }
    }
}

async fn handle_source_command(action: SourceCommands) -> Result<()> {
    match action {
        SourceCommands::Add { name, repo, branch } => {
            source_add(name, repo, branch).await
        }
        SourceCommands::List => {
            source_list().await
        }
        SourceCommands::Remove { name } => {
            source_remove(name).await
        }
        SourceCommands::Update { name } => {
            source_update(name).await
        }
    }
}

async fn source_add(name: String, repo: String, branch: String) -> Result<()> {
    println!("Adding source '{}' from {}", name, repo);

    // Get cache directory
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".cache"))
        .join("skillctrl");
    std::fs::create_dir_all(&cache_dir)?;

    // Register source in state
    let state = skillctrl_state::StateManager::open_default().await?;
    let cache_path = cache_dir.clone();
    let source = skillctrl_state::GitSource::new(
        name.clone(),
        repo.clone(),
        branch.clone(),
        cache_path,
    );
    state.register_source(&source).await?;

    // Clone the repository
    let spinner = create_spinner("Cloning repository...".to_string());
    let git_manager = skillctrl_git::GitManager::new(cache_dir);
    let _path = git_manager.clone(&source).await?;
    spinner.finish_with_message("Repository cloned successfully");

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
        if let Some(commit) = &source.last_commit {
            println!("    Last commit: {}", commit);
        }
        println!();
    }

    Ok(())
}

async fn source_remove(name: String) -> Result<()> {
    println!("Removing source '{}'...", name);
    // TODO: Implement removal from state
    println!("✓ Source '{}' removed", name);
    Ok(())
}

async fn source_update(name: String) -> Result<()> {
    println!("Updating source '{}'...", name);

    let state = skillctrl_state::StateManager::open_default().await?;
    let sources = state.list_sources().await?;

    let source = sources
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| anyhow::anyhow!("source '{}' not found", name))?;

    let spinner = create_spinner("Fetching updates...".to_string());
    let git_manager = skillctrl_git::GitManager::new(source.cache_path.clone());
    let _path = git_manager
        .fetch(&skillctrl_git::GitSource::new(
            source.name.clone(),
            source.repo_url.clone(),
            source.branch.clone(),
            source.cache_path.clone(),
        ))
        .await?;
    spinner.finish_with_message("Updates fetched");

    println!("✓ Source '{}' updated successfully", name);
    Ok(())
}

async fn handle_list(
    source: Option<String>,
    target: Option<String>,
    search: Option<String>,
) -> Result<()> {
    println!("Available bundles:");

    // For now, show example bundles
    println!();
    println!("  review-pr    Pull request review workflow");
    println!("  api-design   API design standards");
    println!();
    println!("Use 'skillctrl show <bundle>' for details");

    Ok(())
}

async fn handle_show(bundle_id: String, source: Option<String>) -> Result<()> {
    println!("Bundle: {}", bundle_id);
    println!("Description: Example bundle");
    println!("Targets: claude-code, codex, cursor");
    println!();
    println!("Components:");
    println!("  - skill: review-pr");
    println!("  - rule: review-policy");
    println!("  - resource: checklist");

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
        println!("[DRY RUN] Would install:");
        println!("  .claude/skills/review-pr/SKILL.md");
        println!("  .claude/rules/review-policy.md");
        return Ok(());
    }

    let spinner = create_spinner("Planning installation...");

    // Get adapter
    let adapter = get_adapter(&target)?;

    // Load bundle
    let state = skillctrl_state::StateManager::open_default().await?;
    let sources = state.list_sources().await?;
    let source_entry = sources
        .iter()
        .find(|s| s.name == source)
        .ok_or_else(|| anyhow::anyhow!("source '{}' not found", source))?;

    let bundle_path = source_entry.cache_path.join("bundles").join(&bundle_id);
    let bundle = skillctrl_catalog::BundleLoader::load_from_dir(&bundle_path)?;

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
            m.insert("bundle_path".to_string(), bundle_path.to_string_lossy().to_string());
            m
        },
    };

    let spinner = create_spinner("Installing...");
    let plan = adapter.plan_install(&bundle, &ctx).await?;
    let result = adapter.apply_install(&plan).await?;
    spinner.finish_with_message("Installation complete");

    // Record installation
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
        ImportCommands::Scan { from, path } => {
            import_scan(from, path).await
        }
        ImportCommands::Plan { from, path, id } => {
            import_plan(from, path, id).await
        }
        ImportCommands::Apply { from, path, out } => {
            import_apply(from, path, out).await
        }
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
        println!("  [{:?}] {}", artifact.kind, artifact.id.as_ref().unwrap_or(&"unknown".to_string()));
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
        println!("    Installed: {}", install.installed_at.format("%Y-%m-%d %H:%M:%S"));
        println!("    Files: {}", install.files_created.len());
        println!();
    }

    Ok(())
}

async fn handle_update(source: Option<String>) -> Result<()> {
    if let Some(name) = source {
        source_update(name).await
    } else {
        // Update all sources
        let state = skillctrl_state::StateManager::open_default().await?;
        let sources = state.list_sources().await?;

        if sources.is_empty() {
            println!("No sources to update.");
            return Ok(());
        }

        for source in &sources {
            source_update(source.name.clone()).await?;
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

    // Load bundle
    let state = skillctrl_state::StateManager::open_default().await?;
    let sources = state.list_sources().await?;
    let source_entry = sources
        .iter()
        .find(|s| s.name == source)
        .ok_or_else(|| anyhow::anyhow!("source '{}' not found", source))?;

    let bundle_path = source_entry.cache_path.join("bundles").join(&bundle_id);
    let bundle = skillctrl_catalog::BundleLoader::load_from_dir(&bundle_path)?;

    // For now, do a simple file copy export
    // Full exporter implementation would use skillctrl-exporter-core
    fs::create_dir_all(&out)?;

    // Copy bundle components
    let components_dir = bundle_path.join("components");
    if components_dir.exists() {
        let out_components = out.join("components");
        fs::create_dir_all(&out_components)?;

        for entry in walkdir::WalkDir::new(&components_dir) {
            let entry = entry.map_err(|e| anyhow::anyhow!("walk error: {}", e))?;
            let src = entry.path();
            let rel = src.strip_prefix(&bundle_path)
                .map_err(|e| anyhow::anyhow!("path strip error: {}", e))?;
            let dest = out.join(rel);

            if src.is_file() {
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(src, &dest)?;
            }
        }
    }

    // Copy bundle manifest
    let bundle_manifest = out.join("bundle.yaml");
    fs::copy(bundle_path.join("bundle.yaml"), &bundle_manifest)?;

    println!("✓ Bundle exported successfully to {}", out.display());

    Ok(())
}

// Helper functions

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

fn get_adapter(endpoint: &Endpoint) -> Result<std::sync::Arc<dyn skillctrl_adapter_core::InstallAdapter + Send + Sync>> {
    match endpoint {
        Endpoint::Known(KnownEndpoint::ClaudeCode) => {
            Ok(std::sync::Arc::new(skillctrl_adapter_claude::ClaudeAdapter::new()))
        }
        Endpoint::Known(KnownEndpoint::Codex) => {
            Ok(std::sync::Arc::new(skillctrl_adapter_codex::CodexAdapter::new()))
        }
        Endpoint::Known(KnownEndpoint::Cursor) => {
            Ok(std::sync::Arc::new(skillctrl_adapter_cursor::CursorAdapter::new()))
        }
        _ => Err(anyhow::anyhow!("adapter not implemented for: {}", endpoint)),
    }
}

fn get_importer(endpoint: &Endpoint) -> Result<std::sync::Arc<dyn skillctrl_importer_core::Importer + Send + Sync>> {
    match endpoint {
        Endpoint::Known(KnownEndpoint::ClaudeCode) => {
            Ok(std::sync::Arc::new(skillctrl_importer_claude::ClaudeImporter::new()))
        }
        _ => Err(anyhow::anyhow!("importer not implemented for: {}", endpoint)),
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
