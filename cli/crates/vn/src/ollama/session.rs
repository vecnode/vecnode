use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFile {
    pub name: String,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub ts_utc: String,
}

impl SessionFile {
    pub fn load_or_new(path: &Path, name: &str) -> Result<Self> {
        if path.exists() {
            let raw = fs::read_to_string(path)
                .with_context(|| format!("failed to read session file: {}", path.display()))?;
            let parsed = serde_json::from_str::<SessionFile>(&raw)
                .with_context(|| format!("failed to parse session file: {}", path.display()))?;
            Ok(parsed)
        } else {
            Ok(Self {
                name: name.to_string(),
                messages: Vec::new(),
            })
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create session directory: {}", parent.display()))?;
            // Set directory permissions to 0o700 (user only) on Unix
            #[cfg(unix)]
            {
                let perms = fs::Permissions::from_mode(0o700);
                fs::set_permissions(parent, perms)
                    .with_context(|| format!("failed to set session directory permissions (0o700) for: {}", parent.display()))?;
            }
        }

        let content = serde_json::to_string_pretty(self).context("failed to serialize session file")?;
        fs::write(path, content)
            .with_context(|| format!("failed to write session file: {}", path.display()))?;
        
        // Set file permissions to 0o600 (user only) on Unix
        #[cfg(unix)]
        {
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(path, perms)
                .with_context(|| format!("failed to set session file permissions (0o600) for: {}", path.display()))?;
        }

        Ok(())
    }

    pub fn append_user(&mut self, content: String) {
        self.messages.push(Message {
            role: "user".to_string(),
            content,
            ts_utc: Utc::now().to_rfc3339(),
        });
    }

    pub fn append_assistant(&mut self, content: String) {
        self.messages.push(Message {
            role: "assistant".to_string(),
            content,
            ts_utc: Utc::now().to_rfc3339(),
        });
    }

    pub fn to_prompt(&self, system: Option<&str>) -> String {
        let mut out = String::new();

        if let Some(system_prompt) = system {
            out.push_str("System:\n");
            out.push_str(system_prompt);
            out.push_str("\n\n");
        }

        for msg in &self.messages {
            out.push_str(&format!("{}:\n{}\n\n", titlecase_role(&msg.role), msg.content));
        }

        out.push_str("Assistant:\n");
        out
    }
}

pub fn session_path(base_dir: &Path, session_name: &str) -> PathBuf {
    let sanitized = session_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    base_dir.join(format!("{}.json", sanitized))
}

fn titlecase_role(role: &str) -> String {
    let mut chars = role.chars();
    if let Some(first) = chars.next() {
        let mut out = first.to_uppercase().to_string();
        out.push_str(chars.as_str());
        out
    } else {
        "Unknown".to_string()
    }
}
