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

/// Send audio to Groq Whisper API and return the transcript text.
pub async fn transcribe(samples: &[i16], sample_rate: u32, api_key: &str, model: &str) -> Result<String, String> {
    let wav_bytes = encode_wav(samples, sample_rate)?;

    let part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("MIME error: {e}"))?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", model.to_string());

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
