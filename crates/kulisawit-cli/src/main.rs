//! Kulisawit CLI binary.

mod commands;

use clap::{Parser, Subcommand};

use kulisawit_core::TaskId;
use std::path::PathBuf;

/// Kulisawit — plant N parallel AI coding agents per task.
#[derive(Debug, Parser)]
#[command(
    name = "kulisawit",
    version = env!("CARGO_PKG_VERSION"),
    about = "Kulisawit — plant N parallel AI coding agents per task",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print version and exit.
    Version,
    /// Dispatch a batch of attempts for a task.
    Run(RunArgs),
    /// Start the HTTP + SSE server.
    Serve(commands::serve::ServeArgs),
}

#[derive(Debug, clap::Args)]
pub struct RunArgs {
    /// Path to the SQLite database. Will be created if missing.
    #[arg(long)]
    pub db: PathBuf,
    /// Path to the git repository hosting the task.
    #[arg(long)]
    pub repo: PathBuf,
    /// Task id (`TaskId` string).
    #[arg(long)]
    pub task: TaskId,
    /// Registered agent id.
    #[arg(long, default_value = "mock")]
    pub agent: String,
    /// Number of parallel attempts.
    #[arg(long, default_value_t = 1)]
    pub batch: usize,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Version => commands::version::run(),
        Command::Run(args) => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(commands::run::run(args))
        }
        Command::Serve(args) => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(commands::serve::run(args))
        }
    }
}
