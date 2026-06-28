use anyhow::Result;
use std::io::Write as IoWrite;
use std::process::{Command, Stdio};

/// How to authenticate calls to Claude.
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// Call the Anthropic HTTP API directly with this key.
    ApiKey(String),
    /// Route through the `claude --print` CLI (no API key needed).
    ClaudeCli,
}

impl AuthMethod {
    /// Resolve from an explicit key, the ANTHROPIC_API_KEY env var, or the claude CLI.
    /// Returns None if no auth method is available.
    pub fn resolve(explicit_key: Option<String>) -> Option<Self> {
        if let Some(k) = explicit_key.or_else(|| std::env::var("ANTHROPIC_API_KEY").ok()) {
            return Some(Self::ApiKey(k));
        }
        if claude_cli_available() {
            return Some(Self::ClaudeCli);
        }
        None
    }
}

/// Returns true if the `claude` binary is on PATH and responds to --version.
pub fn claude_cli_available() -> bool {
    Command::new("claude")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Send a single-turn prompt to Claude and return the text response.
pub fn call_claude(
    prompt: &str,
    auth: &AuthMethod,
    model: &str,
    max_tokens: u32,
) -> Result<String> {
    match auth {
        AuthMethod::ClaudeCli => call_via_cli(prompt),
        AuthMethod::ApiKey(key) => call_via_api(prompt, key, model, max_tokens),
    }
}

fn call_via_cli(prompt: &str) -> Result<String> {
    let mut child = Command::new("claude")
        .arg("--print")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            anyhow::anyhow!("Failed to start claude CLI: {e}. Install from https://claude.ai/code")
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to write to claude CLI stdin: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| anyhow::anyhow!("claude CLI wait failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "claude CLI exited with status {}: {}",
            output.status,
            stderr.trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn call_via_api(prompt: &str, api_key: &str, model: &str, max_tokens: u32) -> Result<String> {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize)]
    struct Msg {
        role: String,
        content: String,
    }
    #[derive(Serialize)]
    struct Req {
        model: String,
        max_tokens: u32,
        messages: Vec<Msg>,
    }
    #[derive(Deserialize)]
    struct Resp {
        content: Vec<Block>,
    }
    #[derive(Deserialize)]
    struct Block {
        text: String,
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let req = Req {
        model: model.to_string(),
        max_tokens,
        messages: vec![Msg {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let resp: Resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&req)
        .send()
        .map_err(|e| anyhow::anyhow!("Anthropic API request failed: {e}"))?
        .error_for_status()
        .map_err(|e| anyhow::anyhow!("Anthropic API error: {e}"))?
        .json()
        .map_err(|e| anyhow::anyhow!("Anthropic API response parse error: {e}"))?;

    Ok(resp
        .content
        .into_iter()
        .map(|b| b.text)
        .collect::<Vec<_>>()
        .join(""))
}

/// Extract a JSON value from a response that may contain markdown code fences.
pub fn extract_json(text: &str) -> &str {
    // Try ```json\n...\n```
    if let Some(start) = text.find("```json\n") {
        let body = &text[start + 8..];
        if let Some(end) = body.find("\n```") {
            return &body[..end];
        }
    }
    // Try ```\n...\n```
    if let Some(start) = text.find("```\n") {
        let body = &text[start + 4..];
        if let Some(end) = body.find("\n```") {
            return &body[..end];
        }
    }
    text.trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_from_fence() {
        let input = "Here is the result:\n```json\n[{\"id\":\"x\"}]\n```\nEnd.";
        assert_eq!(extract_json(input), "[{\"id\":\"x\"}]");
    }

    #[test]
    fn extract_json_bare() {
        let input = "  [1, 2, 3]  ";
        assert_eq!(extract_json(input), "[1, 2, 3]");
    }
}
