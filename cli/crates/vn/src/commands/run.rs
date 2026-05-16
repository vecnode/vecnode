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
    let is_linux = cfg!(target_os = "linux");
    let target = match key.as_str() {
        "ubuntu22" => ScriptTarget {
            relative_path: "scripts/ubuntu22/main.sh",
        },
        "ubuntu22-check-local-network" => ScriptTarget {
            relative_path: "scripts/ubuntu22/check_local_network.sh",
        },
        "ubuntu22-check-internet" => ScriptTarget {
            relative_path: "scripts/ubuntu22/check_internet.sh",
        },
        "ubuntu22-check-dependencies" => ScriptTarget {
            relative_path: "scripts/ubuntu22/check_dependencies.sh",
        },
        "ubuntu22-check-docker" => ScriptTarget {
            relative_path: "scripts/ubuntu22/check_docker.sh",
        },
        "ubuntu22-remove-containers" => ScriptTarget {
            relative_path: "scripts/ubuntu22/remove_containers.sh",
        },
        "ubuntu22-remove-images" => ScriptTarget {
            relative_path: "scripts/ubuntu22/remove_images.sh",
        },
        "ubuntu22-download-all-repos" => ScriptTarget {
            relative_path: "scripts/ubuntu22/download_all_repos.sh",
        },
        "ubuntu22-download-all-orgs" => ScriptTarget {
            relative_path: "scripts/ubuntu22/download_all_orgs.sh",
        },
        "ubuntu22-open-docker" => ScriptTarget {
            relative_path: "scripts/ubuntu22/open_docker.sh",
        },
        "ubuntu22-open-docs" => ScriptTarget {
            relative_path: "scripts/ubuntu22/open_docs.sh",
        },
        "ubuntu22-open-media-processor" => ScriptTarget {
            relative_path: "scripts/ubuntu22/run_cli_container.sh",
        },
        "ubuntu22-run-cli-container" => ScriptTarget {
            relative_path: "scripts/ubuntu22/run_cli_container.sh",
        },
        "ubuntu22-run-silverbullet" => ScriptTarget {
            relative_path: "scripts/ubuntu22/run_silverbullet.sh",
        },
        "ubuntu22-open-silverbullet" => ScriptTarget {
            relative_path: "scripts/ubuntu22/run_silverbullet.sh",
        },
        "ubuntu22-check-ollama" => ScriptTarget {
            relative_path: "scripts/ubuntu22/check_ollama.sh",
        },
        "ubuntu22-open-ollama" => ScriptTarget {
            relative_path: "scripts/ubuntu22/open_ollama.sh",
        },
        "silverbullet" => ScriptTarget {
            relative_path: if cfg!(windows) {
                "scripts/win11/run_silverbullet.bat"
            } else {
                "scripts/ubuntu22/run_silverbullet.sh"
            },
        },
        "check-internet" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/check_internet.sh"
            } else {
                "scripts/win11/check_internet.bat"
            },
        },
        "check-dependencies" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/check_dependencies.sh"
            } else {
                "scripts/win11/check_dependencies.bat"
            },
        },
        "check-local-network" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/check_local_network.sh"
            } else {
                "scripts/win11/check_local_network.bat"
            },
        },
        "check-docker" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/check_docker.sh"
            } else {
                "scripts/win11/check_docker.bat"
            },
        },
        "remove-containers" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/remove_containers.sh"
            } else {
                "scripts/win11/remove_containers.bat"
            },
        },
        "remove-images" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/remove_images.sh"
            } else {
                "scripts/win11/remove_images.bat"
            },
        },
        "open-docker" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/open_docker.sh"
            } else {
                "scripts/win11/open_docker.bat"
            },
        },
        "open-docs" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/open_docs.sh"
            } else {
                "scripts/win11/open_docs.bat"
            },
        },
        "open-media-processor" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/open_media_processor.sh"
            } else {
                "scripts/win11/open_media_processor.bat"
            },
        },
        "download-all-repos" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/download_all_repos.sh"
            } else {
                "scripts/win11/download_all_repos.bat"
            },
        },
        "download-all-orgs" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/download_all_orgs.sh"
            } else {
                "scripts/win11/download_all_orgs.bat"
            },
        },
        "run-cli-container" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/run_cli_container.sh"
            } else {
                "scripts/win11/run_cli_container.bat"
            },
        },
        "open-silverbullet" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/run_silverbullet.sh"
            } else {
                "scripts/win11/run_silverbullet.bat"
            },
        },
        "check-ollama" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/check_ollama.sh"
            } else {
                "scripts/win11/check_ollama.bat"
            },
        },
        "open-ollama" => ScriptTarget {
            relative_path: if is_linux {
                "scripts/ubuntu22/open_ollama.sh"
            } else {
                "scripts/win11/open_ollama.bat"
            },
        },
        "win11" => ScriptTarget {
            relative_path: "scripts/win11/main.bat",
        },
        "win11-check-internet" => ScriptTarget {
            relative_path: "scripts/win11/check_internet.bat",
        },
        "win11-check-local-network" => ScriptTarget {
            relative_path: "scripts/win11/check_local_network.bat",
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
        "win11-remove-containers" => ScriptTarget {
            relative_path: "scripts/win11/remove_containers.bat",
        },
        "win11-remove-images" => ScriptTarget {
            relative_path: "scripts/win11/remove_images.bat",
        },
        "win11-download-all-orgs" => ScriptTarget {
            relative_path: "scripts/win11/download_all_orgs.bat",
        },
        "win11-run-cli-container" => ScriptTarget {
            relative_path: "scripts/win11/run_cli_container.bat",
        },
        "win11-open-silverbullet" => ScriptTarget {
            relative_path: "scripts/win11/run_silverbullet.bat",
        },
        "win11-open-media-processor" => ScriptTarget {
            relative_path: "scripts/win11/open_media_processor.bat",
        },
        "win11-check-ollama" => ScriptTarget {
            relative_path: "scripts/win11/check_ollama.bat",
        },
        "win11-open-ollama" => ScriptTarget {
            relative_path: "scripts/win11/open_ollama.bat",
        },
        "tools-alpine" => ScriptTarget {
            relative_path: "scripts/tools-cli/alpine/main.sh",
        },
        _ => bail!(
            "unknown script name '{}'. Supported linux and win11 script names plus cross-platform aliases (e.g. check-internet, check-dependencies, open-docker, open-docs, open-media-processor, check-ollama, open-ollama, download-all-repos, download-all-orgs, run-cli-container, open-silverbullet)",
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
        let p = PathBuf::from(&path);
        if is_valid_repo_root(&p) {
            return Ok(p);
        } else {
            eprintln!(
                "WARNING: VECNODE_REPO_ROOT={} does not look like a valid vecnode repository (missing .git directory)",
                path
            );
        }
    }

    if let Some(from_config) = loaded.path.parent().and_then(Path::parent) {
        if is_valid_repo_root(from_config) {
            return Ok(from_config.to_path_buf());
        }
    }

    let cwd = env::current_dir().context("failed to read current directory")?;
    for ancestor in cwd.ancestors() {
        if is_valid_repo_root(ancestor) {
            return Ok(ancestor.to_path_buf());
        }
    }

    bail!(
        "could not locate repository root. Run inside the vecnode repo or set VECNODE_REPO_ROOT"
    )
}

/// Validate that a path is a vecnode repository by checking for .git directory and scripts subdirectory.
fn is_valid_repo_root(path: &Path) -> bool {
    let has_git = path.join(".git").exists();
    let has_scripts = path.join("scripts").exists();
    has_git && has_scripts
}
