mod api;
mod config;
mod dto;
mod error;
mod events;
mod state;
mod watch;

use clap::{Parser, Subcommand};
use config::Config;
use org_storage::Workspace;
use state::AppState;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "orgion", version, about = "Org-mode based local-first workspace platform")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Register a directory of .org files as a workspace and build the
    /// initial index. Writes `orgion.toml` in the current directory.
    Init {
        dir: PathBuf,
        #[arg(long, default_value = "orgion.toml")]
        config: PathBuf,
    },
    /// Start the HTTP+WebSocket server.
    Serve {
        #[arg(long, default_value = "orgion.toml")]
        config: PathBuf,
    },
    /// Rebuild the index from the files on disk (safe to run any time;
    /// the index is always a disposable cache — docs/architecture.md §5).
    Index {
        #[arg(long, default_value = "orgion.toml")]
        config: PathBuf,
    },
    /// Parse every .org file in the workspace and report issues, without
    /// touching the index.
    Check {
        #[arg(long, default_value = "orgion.toml")]
        config: PathBuf,
    },
    /// Check + compare the index against the files on disk; suggests
    /// `orgion index` if they've drifted.
    Doctor {
        #[arg(long, default_value = "orgion.toml")]
        config: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Init { dir, config } => cmd_init(dir, config).await,
        Command::Serve { config } => cmd_serve(config).await,
        Command::Index { config } => cmd_index(config).await,
        Command::Check { config } => cmd_check(config).await,
        Command::Doctor { config } => cmd_doctor(config).await,
    }
}

async fn cmd_init(dir: PathBuf, config_path: PathBuf) -> anyhow::Result<()> {
    std::fs::create_dir_all(&dir)?;
    let abs_dir = dir.canonicalize()?;

    if config_path.exists() {
        anyhow::bail!(
            "{} already exists; remove it first if you want to re-init",
            config_path.display()
        );
    }
    Config::write_default(&config_path, &abs_dir)?;
    println!("Wrote {}", config_path.display());

    let config = Config::load(&config_path)?;
    let workspace = Workspace::open(&config.workspace.root)?;
    let pool = org_index::connect(&config.database.url).await?;
    org_index::reindex_workspace(&pool, &workspace, &config.workspace.key).await?;

    let files = org_index::queries::list_files(&pool, &config.workspace.key).await?;
    println!(
        "Indexed {} workspace at {} ({} .org file(s)).",
        config.workspace.key,
        abs_dir.display(),
        files.len()
    );
    println!("Run `orgion serve` to start the server.");
    Ok(())
}

async fn cmd_serve(config_path: PathBuf) -> anyhow::Result<()> {
    let config = load_config_or_explain(&config_path)?;
    let workspace = Workspace::open(&config.workspace.root)?;
    let pool = org_index::connect(&config.database.url).await?;

    tracing::info!(root = %config.workspace.root.display(), "reindexing workspace");
    org_index::reindex_workspace(&pool, &workspace, &config.workspace.key).await?;

    let state = AppState::new(pool, workspace, config.workspace.key.clone());

    let _watcher = if config.index.watch {
        Some(watch::start_watching(state.clone(), tokio::runtime::Handle::current())?)
    } else {
        None
    };

    let app = api::router(state);
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "orgion listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}

async fn cmd_index(config_path: PathBuf) -> anyhow::Result<()> {
    let config = load_config_or_explain(&config_path)?;
    let workspace = Workspace::open(&config.workspace.root)?;
    let pool = org_index::connect(&config.database.url).await?;
    org_index::reindex_workspace(&pool, &workspace, &config.workspace.key).await?;
    let files = org_index::queries::list_files(&pool, &config.workspace.key).await?;
    let mut node_count = 0;
    for f in &files {
        node_count += org_index::queries::list_nodes_for_file(&pool, &f.id).await?.len();
    }
    println!("Indexed {} file(s), {} node(s).", files.len(), node_count);
    Ok(())
}

async fn cmd_check(config_path: PathBuf) -> anyhow::Result<()> {
    let config = load_config_or_explain(&config_path)?;
    let workspace = Workspace::open(&config.workspace.root)?;
    let files = workspace.list_org_files()?;
    let mut issues = 0;
    for rel in &files {
        let content = workspace.read(rel)?;
        let parsed = org_parser::parse(&content.contents);
        for node in parsed.doc.iter() {
            if node
                .body
                .lines()
                .any(|l| l.trim() == ":PROPERTIES:")
            {
                println!(
                    "warning: {}: heading {:?} has a \":PROPERTIES:\" line in its body \
                     (likely meant as a drawer, but a blank line or other separator \
                     before it stopped it from being recognized as one — see \
                     docs/org-mapping.md §4a)",
                    rel, node.title
                );
                issues += 1;
            }
        }
    }
    println!("Checked {} file(s), {} issue(s) found.", files.len(), issues);
    Ok(())
}

async fn cmd_doctor(config_path: PathBuf) -> anyhow::Result<()> {
    cmd_check(config_path.clone()).await?;

    let config = load_config_or_explain(&config_path)?;
    let workspace = Workspace::open(&config.workspace.root)?;
    let on_disk: std::collections::BTreeSet<String> = workspace
        .list_org_files()?
        .iter()
        .map(|r| r.to_string())
        .collect();

    let pool = org_index::connect(&config.database.url).await?;
    let indexed: std::collections::BTreeSet<String> =
        org_index::queries::list_files(&pool, &config.workspace.key)
            .await?
            .into_iter()
            .map(|f| f.path)
            .collect();

    if on_disk == indexed {
        println!("Index is in sync with the workspace on disk.");
    } else {
        println!("Index has drifted from the workspace on disk:");
        for only_disk in on_disk.difference(&indexed) {
            println!("  on disk but not indexed: {only_disk}");
        }
        for only_index in indexed.difference(&on_disk) {
            println!("  indexed but missing on disk: {only_index}");
        }
        println!("Run `orgion index` to rebuild.");
    }
    Ok(())
}

fn load_config_or_explain(path: &PathBuf) -> anyhow::Result<Config> {
    if !path.exists() {
        anyhow::bail!(
            "{} not found. Run `orgion init <dir>` first, or pass --config.",
            path.display()
        );
    }
    Config::load(path)
}
