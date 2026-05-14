mod commands;
mod config;
mod tui;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "vn", version, about = "vecnode cross-platform personal CLI")]
struct Cli {
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Ai(AiArgs),
    Sys(SysArgs),
    Docker(DockerArgs),
    Git(GitArgs),
    Run(RunArgs),
    Tui,
}

#[derive(clap::Args, Debug)]
struct AiArgs {
    prompt: Option<String>,

    #[arg(long)]
    model: Option<String>,

    #[arg(long)]
    host: Option<String>,

    #[arg(long)]
    session: Option<String>,

    #[arg(long)]
    system: Option<String>,

    #[arg(long, action = ArgAction::SetTrue)]
    stream: bool,

    #[arg(long = "no-stream", action = ArgAction::SetTrue)]
    no_stream: bool,
}

#[derive(clap::Args, Debug)]
struct RunArgs {
    name: String,
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
    let loaded = config::load_or_init(cli.config)?;

    match cli.command {
        Some(Command::Ai(args)) => commands::ai::run(args, &loaded).await?,
        Some(Command::Sys(args)) => commands::sys::run(args)?,
        Some(Command::Docker(args)) => commands::docker::run(args, &loaded)?,
        Some(Command::Git(args)) => commands::git::run(args)?,
        Some(Command::Run(args)) => commands::run::run(args, &loaded)?,
        Some(Command::Tui) | None => tui::app::run()?,
    }

    Ok(())
}
