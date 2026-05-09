use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct GenerateChunk {
    response: Option<String>,
    done: Option<bool>,
    error: Option<String>,
}

pub async fn generate<F>(
    client: &Client,
    host: &str,
    model: &str,
    prompt: &str,
    stream: bool,
    mut on_token: F,
) -> Result<String>
where
    F: FnMut(&str),
{
    let url = format!("{}/api/generate", host.trim_end_matches('/'));
    let body = GenerateRequest { model, prompt, stream };

    let response = client
        .post(url)
        .json(&body)
        .send()
        .await
        .context("failed to send request to Ollama")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_else(|_| "<no body>".to_string());
        return Err(anyhow!("Ollama API error {}: {}", status, text));
    }

    if stream {
        let mut output = String::new();
        let mut line_buffer = String::new();
        let mut body_stream = response.bytes_stream();

        while let Some(chunk) = body_stream.next().await {
            let bytes = chunk.context("failed to read streaming body chunk")?;
            line_buffer.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(idx) = line_buffer.find('\n') {
                let line = line_buffer[..idx].trim().to_string();
                line_buffer.drain(..=idx);
                if line.is_empty() {
                    continue;
                }

                let decoded: GenerateChunk = serde_json::from_str(&line)
                    .with_context(|| format!("invalid NDJSON line from Ollama: {}", line))?;

                if let Some(err) = decoded.error {
                    return Err(anyhow!("Ollama reported error: {}", err));
                }

                if let Some(token) = decoded.response {
                    on_token(&token);
                    output.push_str(&token);
                }

                if decoded.done.unwrap_or(false) {
                    return Ok(output);
                }
            }
        }

        if !line_buffer.trim().is_empty() {
            let decoded: GenerateChunk = serde_json::from_str(line_buffer.trim())
                .context("invalid final NDJSON line from Ollama")?;
            if let Some(err) = decoded.error {
                return Err(anyhow!("Ollama reported error: {}", err));
            }
            if let Some(token) = decoded.response {
                on_token(&token);
                output.push_str(&token);
            }
        }

        Ok(output)
    } else {
        let decoded: GenerateChunk = response
            .json()
            .await
            .context("failed to parse non-streaming Ollama response")?;

        if let Some(err) = decoded.error {
            return Err(anyhow!("Ollama reported error: {}", err));
        }

        Ok(decoded.response.unwrap_or_default())
    }
}
