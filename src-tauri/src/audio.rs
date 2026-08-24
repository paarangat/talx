use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream};
use std::sync::{Arc, Mutex};

pub struct AudioCapture {
    stream: Option<Stream>,
    buffer: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
}

#[derive(serde::Serialize)]
pub struct InputDevice {
    pub name: String,
    pub is_default: bool,
}

/// Names of the available input devices. cpal device names are the only
/// stable-ish handle we get, so they double as the identifier.
pub fn list_input_devices() -> Result<Vec<InputDevice>, String> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());

    Ok(host
        .input_devices()
        .map_err(|e| format!("Failed to list input devices: {e}"))?
        // ponytail: a device whose name() errors is unusable as a saved choice, so skip it
        .filter_map(|d| d.name().ok())
        .map(|name| InputDevice {
            is_default: Some(&name) == default_name.as_ref(),
            name,
        })
        .collect())
}

fn find_input_device(host: &cpal::Host, name: &str) -> Option<cpal::Device> {
    host.input_devices()
        .ok()?
        .find(|d| d.name().is_ok_and(|n| n == name))
}

impl AudioCapture {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            stream: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 16000,
        })
    }

    /// `preferred` is a device name from `list_input_devices`; empty means system default.
    /// A saved device that is gone (headset unplugged, renamed) must never cost the user
    /// their dictation — fall back to the default and report it in the returned notice.
    pub fn start(
        &mut self,
        preferred: &str,
    ) -> Result<(Arc<Mutex<Vec<i16>>>, Option<String>), String> {
        let host = cpal::default_host();

        let mut fallback_notice = None;
        let selected = if preferred.is_empty() {
            None
        } else {
            // A device that is listed but cannot report a config (asleep Bluetooth, in use
            // elsewhere) is treated like a missing one — falling back beats losing dictation.
            let found = find_input_device(&host, preferred)
                .filter(|device| device.default_input_config().is_ok());
            if found.is_none() {
                fallback_notice = Some(format!(
                    "Microphone \"{preferred}\" is unavailable — using the system default."
                ));
            }
            found
        };

        let device = match selected {
            Some(device) => device,
            None => host
                .default_input_device()
                .ok_or("No input device available")?,
        };

        let default_config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get input config: {e}"))?;

        self.sample_rate = default_config.sample_rate().0;
        let channels = usize::from(default_config.channels());
        let config: cpal::StreamConfig = default_config.clone().into();

        let buffer = Arc::clone(&self.buffer);
        {
            let mut buf = buffer.lock().map_err(|e| e.to_string())?;
            buf.clear();
        }

        let stream = match default_config.sample_format() {
            // Preserve the device's native channel layout and downmix to mono in software.
            // Forcing mono at the stream level breaks on many Windows microphones.
            SampleFormat::I8 => {
                build_stream::<i8>(&device, &config, channels, Arc::clone(&buffer))?
            }
            SampleFormat::I16 => {
                build_stream::<i16>(&device, &config, channels, Arc::clone(&buffer))?
            }
            SampleFormat::I32 => {
                build_stream::<i32>(&device, &config, channels, Arc::clone(&buffer))?
            }
            SampleFormat::I64 => {
                build_stream::<i64>(&device, &config, channels, Arc::clone(&buffer))?
            }
            SampleFormat::U8 => {
                build_stream::<u8>(&device, &config, channels, Arc::clone(&buffer))?
            }
            SampleFormat::U16 => {
                build_stream::<u16>(&device, &config, channels, Arc::clone(&buffer))?
            }
            SampleFormat::U32 => {
                build_stream::<u32>(&device, &config, channels, Arc::clone(&buffer))?
            }
            SampleFormat::U64 => {
                build_stream::<u64>(&device, &config, channels, Arc::clone(&buffer))?
            }
            SampleFormat::F32 => {
                build_stream::<f32>(&device, &config, channels, Arc::clone(&buffer))?
            }
            SampleFormat::F64 => {
                build_stream::<f64>(&device, &config, channels, Arc::clone(&buffer))?
            }
            fmt => return Err(format!("Unsupported sample format: {fmt}")),
        };

        stream
            .play()
            .map_err(|e| format!("Failed to start stream: {e}"))?;
        self.stream = Some(stream);

        Ok((Arc::clone(&self.buffer), fallback_notice))
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

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    buffer: Arc<Mutex<Vec<i16>>>,
) -> Result<Stream, String>
where
    T: Sample + SizedSample + Copy + Send + 'static,
    f32: FromSample<T>,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                write_input_data::<T>(data, channels, &buffer);
            },
            |err| {
                eprintln!("Audio stream error: {err}");
            },
            None,
        )
        .map_err(|e| format!("Failed to build {} stream: {e}", T::FORMAT))
}

fn write_input_data<T>(input: &[T], channels: usize, buffer: &Arc<Mutex<Vec<i16>>>)
where
    T: Sample + Copy,
    f32: FromSample<T>,
{
    if let Ok(mut buf) = buffer.try_lock() {
        if channels <= 1 {
            buf.extend(
                input
                    .iter()
                    .copied()
                    .map(|sample| normalize_sample(f32::from_sample(sample))),
            );
            return;
        }

        for frame in input.chunks(channels) {
            let sum: f32 = frame.iter().copied().map(f32::from_sample).sum();
            buf.push(normalize_sample(sum / frame.len() as f32));
        }
    }
}

fn normalize_sample(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}
