use crate::config::{expand_tilde, LoadedConfig};
use crate::ollama::{client, session};
use crate::AiArgs;
use anyhow::{bail, Context, Result};
use reqwest::Client;
use std::io::{self, Read, Write};

pub async fn run(args: AiArgs, loaded: &LoadedConfig) -> Result<()> {
    let prompt = match args.prompt {
        Some(value) => value,
        None => read_prompt_from_stdin()?,
    };

    let model = args
        .model
        .unwrap_or_else(|| loaded.config.ollama.model.clone());
    let host = args
        .host
        .unwrap_or_else(|| loaded.config.ollama.host.clone());

    let stream = if args.no_stream {
        false
    } else if args.stream {
        true
    } else {
        loaded.config.ollama.stream
    };

    let system_prompt = args
        .system
        .or_else(|| loaded.config.prompts.system.clone());

    let client = Client::builder().build().context("failed to build HTTP client")?;

    if let Some(session_name) = args.session {
        let sessions_base = expand_tilde(&loaded.config.sessions.dir);
        let path = session::session_path(&sessions_base, &session_name);

        let mut convo = session::SessionFile::load_or_new(&path, &session_name)?;
        convo.append_user(prompt);

        let request_prompt = convo.to_prompt(system_prompt.as_deref());

        let response = if stream {
            client::generate(&client, &host, &model, &request_prompt, true, |token| {
                print!("{}", token);
                let _ = io::stdout().flush();
            })
            .await?
        } else {
            client::generate(&client, &host, &model, &request_prompt, false, |_| {}).await?
        };

        if stream {
            println!();
        } else {
            println!("{}", response);
        }

        convo.append_assistant(response);
        convo.save(&path)?;
        return Ok(());
    }

    let request_prompt = if let Some(system) = system_prompt {
        format!("System:\n{}\n\nUser:\n{}\n\nAssistant:\n", system, prompt)
    } else {
        prompt
    };

    let response = if stream {
        client::generate(&client, &host, &model, &request_prompt, true, |token| {
            print!("{}", token);
            let _ = io::stdout().flush();
        })
        .await?
    } else {
        client::generate(&client, &host, &model, &request_prompt, false, |_| {}).await?
    };

    if stream {
        println!();
    } else {
        println!("{}", response);
    }

    Ok(())
}

fn read_prompt_from_stdin() -> Result<String> {
    if atty::is(atty::Stream::Stdin) {
        bail!("missing prompt. Usage: vn ai \"your prompt\" or pipe stdin")
    }

    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .context("failed to read stdin for prompt")?;

    let trimmed = buf.trim().to_string();
    if trimmed.is_empty() {
        bail!("stdin prompt is empty")
    }

    Ok(trimmed)
}
