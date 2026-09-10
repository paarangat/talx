use std::io::Cursor;

#[derive(serde::Deserialize)]
struct GroqTranscriptionResponse {
    text: String,
}

/// Encode i16 PCM samples to WAV bytes in memory.
fn encode_wav(samples: &[i16], sample_rate: u32) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|e| format!("WAV writer error: {e}"))?;
        for &sample in samples {
            writer
                .write_sample(sample)
                .map_err(|e| format!("WAV write error: {e}"))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("WAV finalize error: {e}"))?;
    }

    Ok(cursor.into_inner())
}

/// The non-file multipart fields for a transcription request. Split out from
/// `transcribe` so the conditional fields can be asserted without a live call.
///
/// A field is omitted entirely when empty — an empty `prompt` is not the same
/// as no prompt for Whisper, and an empty `language` is not auto-detect.
fn form_fields(model: &str, dictionary: &[String], language: &str) -> Vec<(&'static str, String)> {
    let mut fields = vec![("model", model.to_string())];

    let prompt = crate::dictionary::joined(dictionary, crate::dictionary::WHISPER_BUDGET);
    if !prompt.is_empty() {
        fields.push(("prompt", prompt));
    }

    if !language.is_empty() {
        fields.push(("language", language.to_string()));
    }

    fields
}

/// Send audio to Groq Whisper API and return the transcript text.
///
/// `dictionary` holds the user's personal terms; they are sent as Whisper's
/// `prompt` field, which biases decoding toward those spellings. `language` is
/// a bare ISO-639-1 code, or empty to let Whisper auto-detect.
pub async fn transcribe(
    samples: &[i16],
    sample_rate: u32,
    api_key: &str,
    model: &str,
    dictionary: &[String],
    language: &str,
) -> Result<String, String> {
    let wav_bytes = encode_wav(samples, sample_rate)?;

    let part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("MIME error: {e}"))?;

    let mut form = reqwest::multipart::Form::new().part("file", part);
    for (name, value) in form_fields(model, dictionary, language) {
        form = form.text(name, value);
    }

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {api_key}"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Groq request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Groq API error ({status}): {body}"));
    }

    let result: GroqTranscriptionResponse = response
        .json()
        .await
        .map_err(|e| format!("Groq response parse error: {e}"))?;

    Ok(result.text)
}

#[cfg(test)]
mod tests {
    use super::form_fields;

    #[test]
    fn language_is_sent_only_when_set() {
        let chosen = form_fields("whisper-large-v3-turbo", &[], "hi");
        assert!(chosen.iter().any(|(k, v)| *k == "language" && v == "hi"));

        let auto = form_fields("whisper-large-v3-turbo", &[], "");
        assert!(!auto.iter().any(|(k, _)| *k == "language"));
    }
}
