use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use std::sync::{Arc, Mutex};

pub struct AudioCapture {
    stream: Option<Stream>,
    buffer: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
}

impl AudioCapture {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            stream: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 16000,
        })
    }

    pub fn start(&mut self) -> Result<Arc<Mutex<Vec<i16>>>, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        let default_config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get input config: {e}"))?;

        self.sample_rate = default_config.sample_rate().0;

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: default_config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer = Arc::clone(&self.buffer);
        {
            let mut buf = buffer.lock().map_err(|e| e.to_string())?;
            buf.clear();
        }

        let write_buffer = Arc::clone(&buffer);
        let err_fn = |err: cpal::StreamError| {
            eprintln!("Audio stream error: {err}");
        };

        let stream = match default_config.sample_format() {
            cpal::SampleFormat::I16 => device
                .build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buf) = write_buffer.try_lock() {
                            buf.extend_from_slice(data);
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Failed to build i16 stream: {e}"))?,
            cpal::SampleFormat::F32 => {
                let write_buffer = Arc::clone(&self.buffer);
                device
                    .build_input_stream(
                        &cpal::StreamConfig {
                            channels: 1,
                            sample_rate: default_config.sample_rate(),
                            buffer_size: cpal::BufferSize::Default,
                        },
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            if let Ok(mut buf) = write_buffer.try_lock() {
                                buf.extend(data.iter().map(|&s| {
                                    (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
                                }));
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| format!("Failed to build f32 stream: {e}"))?
            }
            fmt => return Err(format!("Unsupported sample format: {fmt:?}")),
        };

        stream.play().map_err(|e| format!("Failed to start stream: {e}"))?;
        self.stream = Some(stream);

        Ok(Arc::clone(&self.buffer))
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn stop(&mut self) -> (Vec<i16>, u32) {
        self.stream = None; // Drop stops the stream
        let samples = {
            let mut buf = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *buf)
        };
        (samples, self.sample_rate)
    }
}
