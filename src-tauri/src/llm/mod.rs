pub mod groq;
pub mod openai;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum LlmProvider {
    Groq,
    OpenAi,
}

impl Default for LlmProvider {
    fn default() -> Self {
        Self::Groq
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PolishResult {
    pub polished_text: String,
    pub model: String,
    pub latency_ms: u64,
}

pub const SYSTEM_PROMPT: &str = "You are a transcript polisher. Clean up the following speech-to-text transcript:\n\n\
- Fix grammar, spelling, and punctuation\n\
- Remove filler words (um, uh, like, you know, sort of, basically, actually)\n\
- Fix false starts and repeated words\n\
- Smooth awkward phrasing from speech-to-text artifacts\n\
- Preserve the speaker's voice, intent, and meaning exactly\n\
- Do NOT restructure sentences or add new content\n\
- Do NOT add commentary, explanations, or formatting\n\
- Return ONLY the polished text, nothing else";

pub async fn polish(text: &str, provider: &LlmProvider, api_key: &str, model: &str) -> Result<PolishResult, String> {
    match provider {
        LlmProvider::Groq => groq::polish(text, api_key, model).await,
        LlmProvider::OpenAi => openai::polish(text, api_key, model).await,
    }
}
