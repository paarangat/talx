pub mod groq;
pub mod soniox;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum AsrProvider {
    Groq,
    Soniox,
}

impl Default for AsrProvider {
    fn default() -> Self {
        Self::Groq
    }
}
