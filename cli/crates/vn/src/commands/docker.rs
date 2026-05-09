use crate::config::LoadedConfig;
use crate::{DockerArgs, DockerSubcommand};
use anyhow::{anyhow, Context, Result};
use std::process::{Command, Stdio};

pub fn run(args: DockerArgs, loaded: &LoadedConfig) -> Result<()> {
    match args.command {
        DockerSubcommand::Ps => run_cmd("docker", &["ps"]),
        DockerSubcommand::Prune => run_cmd("docker", &["system", "prune", "-af"]),
        DockerSubcommand::Up { service } => {
            if matches!(service.as_deref(), Some("silverbullet")) {
                crate::commands::run::run_named_script("silverbullet", loaded)
            } else if let Some(name) = service {
                run_cmd("docker", &["start", &name])
            } else {
                println!("Use: vn docker up <service>");
                Ok(())
            }
        }
        DockerSubcommand::Down { service } => {
            if let Some(name) = service {
                run_cmd("docker", &["stop", &name])
            } else {
                println!("Use: vn docker down <service>");
                Ok(())
            }
        }
    }
}

fn run_cmd(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to run: {} {}", program, args.join(" ")))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("command exited with status: {}", status))
    }
}
