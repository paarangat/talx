use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

/// A handle to a running Soniox streaming session.
/// Drop it or call `stop()` to close the WebSocket.
pub struct SonioxSession {
    stop_tx: Option<mpsc::Sender<()>>,
    task: Option<tokio::task::JoinHandle<Result<String, String>>>,
}

#[derive(serde::Serialize)]
struct SonioxConfig {
    api_key: String,
    sample_rate_hertz: u32,
    audio_format: String,
    model: String,
    include_nonfinal: bool,
    /// Biases the model toward a language without restricting it to one. Left
    /// off the wire entirely for auto-detect — Soniox reads an absent hint as
    /// "any supported language", which is not what an empty list would mean.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    language_hints: Vec<String>,
}

impl SonioxConfig {
    /// `language` is a bare ISO-639-1 code, or empty for auto-detect.
    fn new(api_key: String, sample_rate: u32, model: String, language: &str) -> Self {
        Self {
            api_key,
            sample_rate_hertz: sample_rate,
            audio_format: "pcm_s16le".to_string(),
            model,
            include_nonfinal: true,
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
    words: Vec<SonioxWord>,
}

#[derive(serde::Deserialize)]
struct SonioxWord {
    text: String,
    #[serde(default)]
    is_final: bool,
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
        let url = "wss://api.soniox.com/transcribe-websocket";

        let (ws_stream, _) = tokio_tungstenite::connect_async(url)
            .await
            .map_err(|e| format!("Soniox WebSocket connect failed: {e}"))?;

        let (mut ws_write, mut ws_read) = ws_stream.split();

        // Send config message
        let config = SonioxConfig::new(api_key, sample_rate, model, language);
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
                        // Signal end of audio
                        let _ = ws_write.close().await;

                        // Drain remaining responses
                        while let Some(Ok(msg)) = ws_read.next().await {
                            if let Message::Text(text) = msg {
                                if let Ok(resp) = serde_json::from_str::<SonioxResponse>(&text) {
                                    for word in &resp.words {
                                        if word.is_final && !word.text.is_empty() {
                                            if !final_transcript.is_empty() {
                                                final_transcript.push(' ');
                                            }
                                            final_transcript.push_str(&word.text);
                                        }
                                    }
                                }
                            }
                        }

                        return Ok(final_transcript);
                    }

                    // Read incoming transcript messages
                    msg = ws_read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(resp) = serde_json::from_str::<SonioxResponse>(&text) {
                                    let mut current = final_transcript.clone();
                                    for word in &resp.words {
                                        if !word.text.is_empty() {
                                            if !current.is_empty() {
                                                current.push(' ');
                                            }
                                            current.push_str(&word.text);
                                        }
                                        if word.is_final && !word.text.is_empty() {
                                            if !final_transcript.is_empty() {
                                                final_transcript.push(' ');
                                            }
                                            final_transcript.push_str(&word.text);
                                        }
                                    }
                                    let _ = partial_tx.send(current);
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
    use super::SonioxConfig;

    fn config_json(language: &str) -> String {
        let config = SonioxConfig::new("key".to_string(), 16000, "model".to_string(), language);
        serde_json::to_string(&config).expect("config serializes")
    }

    #[test]
    fn language_hints_are_sent_only_when_set() {
        assert!(config_json("hi").contains(r#""language_hints":["hi"]"#));
        assert!(!config_json("").contains("language_hints"));
    }
}
