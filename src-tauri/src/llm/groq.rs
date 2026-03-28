use super::{PolishResult, SYSTEM_PROMPT};
use std::time::Instant;

#[derive(serde::Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(serde::Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(serde::Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(serde::Deserialize)]
struct ChatResponseMessage {
    content: String,
}

#[derive(serde::Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    model: String,
}

pub async fn polish(text: &str, api_key: &str) -> Result<PolishResult, String> {
    let start = Instant::now();

    let request = ChatRequest {
        model: "llama-3.3-70b-versatile".to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: SYSTEM_PROMPT.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: text.to_string(),
            },
        ],
        temperature: 0.3,
        max_tokens: 1024,
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Groq LLM request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Groq LLM API error ({status}): {body}"));
    }

    let result: ChatResponse = response
        .json()
        .await
        .map_err(|e| format!("Groq LLM response parse error: {e}"))?;

    let polished_text = result
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or("Groq LLM returned no choices")?;

    let latency_ms = start.elapsed().as_millis() as u64;

    Ok(PolishResult {
        polished_text,
        model: result.model,
        latency_ms,
    })
}
