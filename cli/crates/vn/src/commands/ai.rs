use crate::AiArgs;
use anyhow::Result;

pub async fn run(_args: AiArgs, loaded: &crate::config::LoadedConfig) -> Result<()> {
    println!(
        "todo: AI functionality is not yet available (configured host: {}, model: {})",
        loaded.config.ollama.host, loaded.config.ollama.model
    );
    Ok(())
}
