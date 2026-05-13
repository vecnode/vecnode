use crate::config::LoadedConfig;
use crate::RunArgs;
use anyhow::{anyhow, bail, Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn run(args: RunArgs, loaded: &LoadedConfig) -> Result<()> {
    run_named_script(&args.name, loaded)
}

pub fn run_named_script(name: &str, loaded: &LoadedConfig) -> Result<()> {
    let repo_root = detect_repo_root(loaded)?;
    let target = map_script(name)?;
    let path = repo_root.join(target.relative_path);

    if !path.exists() {
        bail!("script not found: {}", path.display());
    }

    let status = run_script(&path).with_context(|| format!("failed running script: {}", path.display()))?;
    if !status.success() {
        return Err(anyhow!("script exited with status: {}", status));
    }

    Ok(())
}

struct ScriptTarget {
    relative_path: &'static str,
}

fn map_script(name: &str) -> Result<ScriptTarget> {
    let key = name.to_ascii_lowercase();
    let target = match key.as_str() {
        "ubuntu22" => ScriptTarget {
            relative_path: "scripts/ubuntu22/main.sh",
        },
        "ubuntu22-check-internet" => ScriptTarget {
            relative_path: "scripts/ubuntu22/check_internet.sh",
        },
        "ubuntu22-check-dependencies" => ScriptTarget {
            relative_path: "scripts/ubuntu22/check_dependencies.sh",
        },
        "ubuntu22-download-all-repos" => ScriptTarget {
            relative_path: "scripts/ubuntu22/download_all_repos.sh",
        },
        "ubuntu22-download-all-orgs" => ScriptTarget {
            relative_path: "scripts/ubuntu22/download_all_orgs.sh",
        },
        "ubuntu22-run-cli-container" => ScriptTarget {
            relative_path: "scripts/ubuntu22/run_cli_container.sh",
        },
        "ubuntu22-run-silverbullet" => ScriptTarget {
            relative_path: "scripts/ubuntu22/run_silverbullet.sh",
        },
        "silverbullet" => ScriptTarget {
            relative_path: if cfg!(windows) {
                "scripts/win11/run_silverbullet.bat"
            } else {
                "scripts/ubuntu22/run_silverbullet.sh"
            },
        },
        "win11" => ScriptTarget {
            relative_path: "scripts/win11/main.bat",
        },
        "win11-check-internet" => ScriptTarget {
            relative_path: "scripts/win11/check_internet.bat",
        },
        "win11-check-dependencies" => ScriptTarget {
            relative_path: "scripts/win11/check_dependencies.bat",
        },
        "win11-download-all-repos" => ScriptTarget {
            relative_path: "scripts/win11/download_all_repos.bat",
        },
        "win11-open-docker" => ScriptTarget {
            relative_path: "scripts/win11/open_docker.bat",
        },
        "win11-open-docs" => ScriptTarget {
            relative_path: "scripts/win11/open_docs.bat",
        },
        "win11-check-docker" => ScriptTarget {
            relative_path: "scripts/win11/check_docker.bat",
        },
        "win11-download-all-orgs" => ScriptTarget {
            relative_path: "scripts/win11/download_all_orgs.bat",
        },
        "win11-run-cli-container" => ScriptTarget {
            relative_path: "scripts/win11/run_cli_container.bat",
        },
        "win11-run-silverbullet" => ScriptTarget {
            relative_path: "scripts/win11/run_silverbullet.bat",
        },
        "win11-install-app-wezterm" => ScriptTarget {
            relative_path: "scripts/win11/install_app_wezterm.bat",
        },
        "tools-alpine" => ScriptTarget {
            relative_path: "scripts/tools-cli/alpine/main.sh",
        },
        _ => bail!(
            "unknown script name '{}'. Supported: ubuntu22, ubuntu22-check-internet, ubuntu22-check-dependencies, ubuntu22-download-all-repos, ubuntu22-download-all-orgs, ubuntu22-run-cli-container, ubuntu22-run-silverbullet, win11, win11-check-internet, win11-check-dependencies, win11-download-all-repos, win11-open-docker, win11-open-docs, win11-check-docker, win11-download-all-orgs, win11-run-cli-container, win11-run-silverbullet, win11-install-app-wezterm, tools-alpine, silverbullet",
            name
        ),
    };

    Ok(target)
}

fn run_script(script: &Path) -> Result<std::process::ExitStatus> {
    let ext = script
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut cmd = if ext == "bat" {
        if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(script);
            c
        } else {
            bail!("cannot execute .bat script on non-Windows host")
        }
    } else if cfg!(windows) {
        let mut c = Command::new("wsl");
        c.arg("bash").arg(windows_path_to_wsl(script)?);
        c
    } else {
        let mut c = Command::new("bash");
        c.arg(script);
        c
    };

    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    cmd.status().context("failed to spawn process")
}

fn windows_path_to_wsl(path: &Path) -> Result<String> {
    let raw = path
        .canonicalize()
        .with_context(|| format!("failed to resolve script path: {}", path.display()))?;

    let raw_str = raw
        .to_str()
        .ok_or_else(|| anyhow!("script path is not valid unicode"))?;

    // Canonical Windows paths may use the extended-length prefix (e.g. \\\\?\\C:\\...)
    // which must be removed before drive-letter parsing.
    let normalized = if let Some(stripped) = raw_str.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", stripped)
    } else if let Some(stripped) = raw_str.strip_prefix(r"\\?\") {
        stripped.to_string()
    } else {
        raw_str.to_string()
    };

    if normalized.starts_with(r"\\") {
        bail!(
            "UNC paths are not supported for WSL script execution: {}",
            normalized
        );
    }

    let mut chars = normalized.chars();
    let drive = chars
        .next()
        .ok_or_else(|| anyhow!("invalid script path"))?
        .to_ascii_lowercase();
    let colon = chars.next().ok_or_else(|| anyhow!("invalid script path"))?;

    if !drive.is_ascii_alphabetic() || colon != ':' {
        bail!("unsupported Windows path format: {}", normalized);
    }

    let rest = chars.as_str().replace('\\', "/");
    Ok(format!("/mnt/{}/{}", drive, rest.trim_start_matches('/')))
}

fn detect_repo_root(loaded: &LoadedConfig) -> Result<PathBuf> {
    if let Ok(path) = env::var("VECNODE_REPO_ROOT") {
        let p = PathBuf::from(path);
        if p.join("scripts").exists() {
            return Ok(p);
        }
    }

    if let Some(from_config) = loaded.path.parent().and_then(Path::parent) {
        if from_config.join("scripts").exists() {
            return Ok(from_config.to_path_buf());
        }
    }

    let cwd = env::current_dir().context("failed to read current directory")?;
    for ancestor in cwd.ancestors() {
        if ancestor.join("scripts").exists() {
            return Ok(ancestor.to_path_buf());
        }
    }

    bail!(
        "could not locate repository root. Run inside the vecnode repo or set VECNODE_REPO_ROOT"
    )
}
