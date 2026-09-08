use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

/// Current real-time endpoint. The old `api.soniox.com/transcribe-websocket`
/// answers 502 — it was retired together with the `soniox-v2` model, so every
/// Soniox path in the app has to speak this API.
pub const ENDPOINT: &str = "wss://stt-rt.soniox.com/transcribe-websocket";

/// Recommended real-time model. `stt-rt-v4` is an alias that Soniox removes on
/// 2026-06-30; v5 is what it routes to anyway.
pub const DEFAULT_MODEL: &str = "stt-rt-v5";

/// Saved settings may still carry the retired `soniox-v2`, and the model picker
/// lets users type anything. Only current real-time models go on the wire;
/// everything else falls back to the default rather than failing the session.
pub fn current_model(model: &str) -> String {
    if model.starts_with("stt-rt-") {
        model.to_string()
    } else {
        DEFAULT_MODEL.to_string()
    }
}

/// A handle to a running Soniox streaming session.
/// Drop it or call `stop()` to close the WebSocket.
pub struct SonioxSession {
    stop_tx: Option<mpsc::Sender<()>>,
    task: Option<tokio::task::JoinHandle<Result<String, String>>>,
}

/// The first message on the socket. Field names are the current API's:
/// `sample_rate` + `num_channels` (not `sample_rate_hertz`), and non-final
/// tokens are always streamed — there is no `include_nonfinal` any more.
#[derive(serde::Serialize)]
pub struct SonioxConfig {
    api_key: String,
    model: String,
    audio_format: String,
    sample_rate: u32,
    num_channels: u32,
    /// Biases the model toward a language without restricting it to one. Left
    /// off the wire entirely for auto-detect — Soniox reads an absent hint as
    /// "any supported language", which is not what an empty list would mean.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    language_hints: Vec<String>,
}

impl SonioxConfig {
    /// `language` is a bare ISO-639-1 code, or empty for auto-detect.
    pub fn new(api_key: String, sample_rate: u32, model: &str, language: &str) -> Self {
        Self {
            api_key,
            model: current_model(model),
            audio_format: "pcm_s16le".to_string(),
            sample_rate,
            num_channels: 1,
            language_hints: if language.is_empty() {
                Vec::new()
            } else {
                vec![language.to_string()]
            },
        }
    }
}

#[derive(serde::Deserialize)]
struct SonioxResponse {
    #[serde(default)]
    tokens: Vec<SonioxToken>,
    /// Set on the last message after we send the end-of-audio frame.
    #[serde(default)]
    finished: bool,
    /// Present only on failure (bad key, rejected config, quota). The old
    /// code dropped unparseable messages on the floor, which is how a rejected
    /// config used to look like a session that simply never produced words.
    error_message: Option<String>,
}

#[derive(serde::Deserialize)]
struct SonioxToken {
    text: String,
    #[serde(default)]
    is_final: bool,
}

/// Returns the error a Soniox message carries, if any. Shared with the API-key
/// test in `lib.rs` so both paths read errors the same way.
pub fn error_in(message: &str) -> Option<String> {
    serde_json::from_str::<SonioxResponse>(message)
        .ok()
        .and_then(|r| r.error_message)
}

/// Folds one response into the running transcript.
///
/// Final tokens are appended to `final_transcript` and never sent again;
/// non-final tokens are re-sent on every response until they finalise, so the
/// returned "current" view is `final_transcript` plus this response's tokens.
/// Tokens carry their own leading whitespace and punctuation, so they are
/// concatenated as-is — joining with spaces would double every one.
fn fold_tokens(final_transcript: &mut String, tokens: &[SonioxToken]) -> String {
    let mut current = final_transcript.clone();
    for token in tokens {
        current.push_str(&token.text);
        if token.is_final {
            final_transcript.push_str(&token.text);
        }
    }
    current
}

impl SonioxSession {
    /// Start a streaming session. Returns the session handle.
    /// `partial_tx` sends partial transcript strings to the frontend.
    pub async fn start(
        api_key: String,
        sample_rate: u32,
        audio_buffer: Arc<Mutex<Vec<i16>>>,
        partial_tx: mpsc::UnboundedSender<String>,
        model: String,
        language: &str,
    ) -> Result<Self, String> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(ENDPOINT)
            .await
            .map_err(|e| format!("Soniox WebSocket connect failed: {e}"))?;

        let (mut ws_write, mut ws_read) = ws_stream.split();

        // Send config message
        let config = SonioxConfig::new(api_key, sample_rate, &model, language);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| format!("Config serialize error: {e}"))?;
        ws_write
            .send(Message::Text(config_json.into()))
            .await
            .map_err(|e| format!("Failed to send config: {e}"))?;

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);

        // Track cursor position in the shared buffer
        let task = tokio::spawn(async move {
            let mut read_cursor: usize = 0;
            let mut final_transcript = String::new();

            loop {
                tokio::select! {
                    // Check for stop signal
                    _ = stop_rx.recv() => {
                        // Send any remaining audio
                        let chunk = {
                            let buf = audio_buffer.lock().unwrap_or_else(|e| e.into_inner());
                            if read_cursor < buf.len() {
                                let samples = &buf[read_cursor..];
                                let bytes: Vec<u8> = samples.iter()
                                    .flat_map(|s| s.to_le_bytes())
                                    .collect();
                                Some(bytes)
                            } else {
                                None
                            }
                        };
                        if let Some(bytes) = chunk {
                            let _ = ws_write.send(Message::Binary(bytes.into())).await;
                        }
                        // An empty frame is the end-of-audio signal. Closing the
                        // socket instead would drop the trailing tokens the
                        // server is still finalising.
                        let _ = ws_write.send(Message::Text(String::new().into())).await;

                        // Drain until the server says it is done
                        while let Some(Ok(msg)) = ws_read.next().await {
                            if let Message::Text(text) = msg {
                                if let Ok(resp) = serde_json::from_str::<SonioxResponse>(&text) {
                                    if let Some(err) = resp.error_message {
                                        if final_transcript.is_empty() {
                                            let _ = ws_write.close().await;
                                            return Err(format!("Soniox error: {err}"));
                                        }
                                        eprintln!("Soniox error after partial transcript: {err}");
                                        break;
                                    }
                                    fold_tokens(&mut final_transcript, &resp.tokens);
                                    if resp.finished {
                                        break;
                                    }
                                }
                            }
                        }
                        let _ = ws_write.close().await;

                        return Ok(final_transcript);
                    }

                    // Read incoming transcript messages
                    msg = ws_read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(resp) = serde_json::from_str::<SonioxResponse>(&text) {
                                    if let Some(err) = resp.error_message {
                                        if final_transcript.is_empty() {
                                            return Err(format!("Soniox error: {err}"));
                                        }
                                        eprintln!("Soniox error after partial transcript: {err}");
                                        return Ok(final_transcript);
                                    }
                                    let current = fold_tokens(&mut final_transcript, &resp.tokens);
                                    let _ = partial_tx.send(current);
                                    if resp.finished {
                                        return Ok(final_transcript);
                                    }
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                return Ok(final_transcript);
                            }
                            Some(Err(e)) => {
                                if final_transcript.is_empty() {
                                    return Err(format!("Soniox WebSocket error: {e}"));
                                }
                                // Return what we have on error
                                return Ok(final_transcript);
                            }
                            _ => {}
                        }
                    }

                    // Send audio chunks every 100ms
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                        let chunk = {
                            let buf = audio_buffer.lock().unwrap_or_else(|e| e.into_inner());
                            if read_cursor < buf.len() {
                                let samples = &buf[read_cursor..];
                                read_cursor = buf.len();
                                let bytes: Vec<u8> = samples.iter()
                                    .flat_map(|s| s.to_le_bytes())
                                    .collect();
                                Some(bytes)
                            } else {
                                None
                            }
                        };
                        if let Some(bytes) = chunk {
                            if let Err(e) = ws_write.send(Message::Binary(bytes.into())).await {
                                eprintln!("Soniox send error: {e}");
                                return Ok(final_transcript);
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            stop_tx: Some(stop_tx),
            task: Some(task),
        })
    }

    /// Stop the session and return the final transcript.
    pub async fn stop(&mut self) -> Result<String, String> {
        if let Some(tx) = self.stop_tx.take() {
            let _: Result<(), _> = tx.send(()).await;
        }
        if let Some(task) = self.task.take() {
            let result: Result<String, String> =
                task.await.map_err(|e| format!("Soniox task panicked: {e}"))?;
            result
        } else {
            Ok(String::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{current_model, error_in, fold_tokens, SonioxConfig, SonioxResponse, DEFAULT_MODEL};

    fn config_json(model: &str, language: &str) -> String {
        let config = SonioxConfig::new("key".to_string(), 16000, model, language);
        serde_json::to_string(&config).expect("config serializes")
    }

    #[test]
    fn config_uses_current_api_field_names() {
        let json = config_json(DEFAULT_MODEL, "");
        assert!(json.contains(r#""sample_rate":16000"#));
        assert!(json.contains(r#""num_channels":1"#));
        assert!(json.contains(r#""audio_format":"pcm_s16le""#));
        assert!(!json.contains("sample_rate_hertz"));
        assert!(!json.contains("include_nonfinal"));
    }

    #[test]
    fn language_hints_are_sent_only_when_set() {
        assert!(config_json(DEFAULT_MODEL, "hi").contains(r#""language_hints":["hi"]"#));
        assert!(!config_json(DEFAULT_MODEL, "").contains("language_hints"));
    }

    #[test]
    fn retired_models_fall_back_to_the_default() {
        assert_eq!(current_model("soniox-v2"), DEFAULT_MODEL);
        assert_eq!(current_model(""), DEFAULT_MODEL);
        assert_eq!(current_model("stt-rt-v4"), "stt-rt-v4");
        assert!(config_json("soniox-v2", "").contains(r#""model":"stt-rt-v5""#));
    }

    #[test]
    fn tokens_concatenate_without_added_spaces() {
        let resp: SonioxResponse = serde_json::from_str(
            r#"{"tokens":[{"text":"Hello","is_final":true},{"text":",","is_final":true},{"text":" wor","is_final":false}]}"#,
        )
        .expect("parses");
        let mut final_transcript = String::new();
        let current = fold_tokens(&mut final_transcript, &resp.tokens);
        assert_eq!(current, "Hello, wor");
        assert_eq!(final_transcript, "Hello,");
    }

    #[test]
    fn error_messages_are_surfaced() {
        assert_eq!(
            error_in(r#"{"error_code":401,"error_message":"Invalid API key"}"#),
            Some("Invalid API key".to_string())
        );
        assert_eq!(error_in(r#"{"tokens":[],"finished":true}"#), None);
        assert_eq!(error_in("not json"), None);
    }
}
