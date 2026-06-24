mod commands;
mod config;
mod ollama;
mod tray;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "vn", version, about = "vecnode cross-platform personal CLI")]
struct Cli {
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[arg(long, global = true, hide = true)]
    repo_root: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Ai(AiArgs),
    Sys(SysArgs),
    Docker(DockerArgs),
    Git(GitArgs),
    Net(NetArgs),
    Run(RunArgs),
    Tray,
    Tui,
}

#[derive(clap::Args, Debug)]
struct AiArgs {
    #[command(subcommand)]
    command: AiCommand,

    /// Ollama host URL (defaults to the configured host).
    #[arg(long, global = true)]
    host: Option<String>,
}

#[derive(Subcommand, Debug)]
enum AiCommand {
    /// Check whether the Ollama server is reachable.
    Status,
    /// List locally installed models (one name per line).
    Models,
    /// Download a model by name (e.g. llama3.2).
    Pull { name: String },
    /// Send a chat message and print the reply (context kept per session).
    Chat {
        /// The message to send (pass as a single quoted argument).
        message: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        system: Option<String>,
    },
}

#[derive(clap::Args, Debug)]
struct RunArgs {
    name: String,
}

#[derive(clap::Args, Debug)]
struct NetArgs {
    #[command(subcommand)]
    command: Option<NetSubcommand>,
}

#[derive(Subcommand, Debug)]
enum NetSubcommand {
    /// Scan open ports with RustScan. Defaults to the local /24 subnet.
    Scan {
        /// Target to scan: IP, CIDR, or hostname. Defaults to the local /24 subnet.
        target: Option<String>,
    },
}

#[derive(clap::Args, Debug)]
struct SysArgs {
    #[command(subcommand)]
    command: Option<SysSubcommand>,
}

#[derive(Subcommand, Debug)]
enum SysSubcommand {
    Info,
    Update,
    Clean,
}

#[derive(clap::Args, Debug)]
struct DockerArgs {
    #[command(subcommand)]
    command: DockerSubcommand,
}

#[derive(Subcommand, Debug)]
enum DockerSubcommand {
    Ps,
    Up { service: Option<String> },
    Down { service: Option<String> },
    Prune,
}

#[derive(clap::Args, Debug)]
struct GitArgs {
    #[command(subcommand)]
    command: GitSubcommand,
}

#[derive(Subcommand, Debug)]
enum GitSubcommand {
    Sync { #[arg(long)] root: Option<PathBuf> },
    Status { #[arg(long)] root: Option<PathBuf> },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo_root = cli.repo_root.clone();
    let loaded = config::load_or_init(cli.config)?;

    match cli.command {
        Some(Command::Ai(args)) => commands::ai::run(args, &loaded).await?,
        Some(Command::Sys(args)) => commands::sys::run(args)?,
        Some(Command::Docker(args)) => commands::docker::run(args, &loaded)?,
        Some(Command::Git(args)) => commands::git::run(args)?,
        Some(Command::Net(args)) => commands::net::run(args)?,
        Some(Command::Run(args)) => commands::run::run(args, &loaded)?,
        Some(Command::Tray) => tray::run(repo_root)?,
        Some(Command::Tui) | None => tui::app::run(repo_root)?,
    }

    Ok(())
}
