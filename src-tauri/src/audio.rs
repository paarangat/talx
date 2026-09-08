use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

pub struct AudioCapture {
    stream: Option<Stream>,
    buffer: Arc<Mutex<Vec<i16>>>,
    /// Latest input loudness, 0.0..=1.0, as `f32::to_bits`. An atomic rather
    /// than a mutex because it is written from the realtime audio callback,
    /// where blocking on a poisoned or contended lock would drop samples.
    level: Arc<AtomicU32>,
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
            level: Arc::new(AtomicU32::new(0)),
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
        let level = Arc::clone(&self.level);
        self.level.store(0, Ordering::Relaxed);
        {
            let mut buf = buffer.lock().map_err(|e| e.to_string())?;
            buf.clear();
        }

        let stream = match default_config.sample_format() {
            // Preserve the device's native channel layout and downmix to mono in software.
            // Forcing mono at the stream level breaks on many Windows microphones.
            SampleFormat::I8 => {
                build_stream::<i8>(&device, &config, channels, Arc::clone(&buffer), Arc::clone(&level))?
            }
            SampleFormat::I16 => {
                build_stream::<i16>(&device, &config, channels, Arc::clone(&buffer), Arc::clone(&level))?
            }
            SampleFormat::I32 => {
                build_stream::<i32>(&device, &config, channels, Arc::clone(&buffer), Arc::clone(&level))?
            }
            SampleFormat::I64 => {
                build_stream::<i64>(&device, &config, channels, Arc::clone(&buffer), Arc::clone(&level))?
            }
            SampleFormat::U8 => {
                build_stream::<u8>(&device, &config, channels, Arc::clone(&buffer), Arc::clone(&level))?
            }
            SampleFormat::U16 => {
                build_stream::<u16>(&device, &config, channels, Arc::clone(&buffer), Arc::clone(&level))?
            }
            SampleFormat::U32 => {
                build_stream::<u32>(&device, &config, channels, Arc::clone(&buffer), Arc::clone(&level))?
            }
            SampleFormat::U64 => {
                build_stream::<u64>(&device, &config, channels, Arc::clone(&buffer), Arc::clone(&level))?
            }
            SampleFormat::F32 => {
                build_stream::<f32>(&device, &config, channels, Arc::clone(&buffer), Arc::clone(&level))?
            }
            SampleFormat::F64 => {
                build_stream::<f64>(&device, &config, channels, Arc::clone(&buffer), Arc::clone(&level))?
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

    /// Shared handle to the live input level so the widget can render a
    /// waveform driven by the actual microphone rather than by noise.
    pub fn level(&self) -> Arc<AtomicU32> {
        Arc::clone(&self.level)
    }

    pub fn stop(&mut self) -> (Vec<i16>, u32) {
        self.stream = None; // Drop stops the stream
        self.level.store(0, Ordering::Relaxed);
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
    level: Arc<AtomicU32>,
) -> Result<Stream, String>
where
    T: Sample + SizedSample + Copy + Send + 'static,
    f32: FromSample<T>,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                write_input_data::<T>(data, channels, &buffer, &level);
            },
            |err| {
                eprintln!("Audio stream error: {err}");
            },
            None,
        )
        .map_err(|e| format!("Failed to build {} stream: {e}", T::FORMAT))
}

fn write_input_data<T>(
    input: &[T],
    channels: usize,
    buffer: &Arc<Mutex<Vec<i16>>>,
    level: &Arc<AtomicU32>,
) where
    T: Sample + Copy,
    f32: FromSample<T>,
{
    store_level(input, level);

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

/// RMS of the callback's frames, mapped onto a perceptual 0..=1 range.
///
/// Speech RMS sits far below full scale (roughly -30..-12 dBFS), so raw RMS
/// would keep the waveform pinned near zero. The dBFS window below is what
/// makes normal talking fill the bars without clipping on a loud room.
fn store_level<T>(input: &[T], level: &Arc<AtomicU32>)
where
    T: Sample + Copy,
    f32: FromSample<T>,
{
    if input.is_empty() {
        return;
    }

    let sum_squares: f32 = input
        .iter()
        .copied()
        .map(|s| {
            let v = f32::from_sample(s);
            v * v
        })
        .sum();
    let rms = (sum_squares / input.len() as f32).sqrt();

    const FLOOR_DB: f32 = -60.0;
    const CEIL_DB: f32 = -12.0;
    let db = 20.0 * rms.max(1e-7).log10();
    let scaled = ((db - FLOOR_DB) / (CEIL_DB - FLOOR_DB)).clamp(0.0, 1.0);

    level.store(scaled.to_bits(), Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(level: &Arc<AtomicU32>) -> f32 {
        f32::from_bits(level.load(Ordering::Relaxed))
    }

    #[test]
    fn level_tracks_loudness() {
        let level = Arc::new(AtomicU32::new(0));

        store_level(&[0.0f32; 128], &level);
        let silence = read(&level);

        store_level(&[0.05f32; 128], &level);
        let quiet = read(&level);

        store_level(&[0.9f32; 128], &level);
        let loud = read(&level);

        assert_eq!(silence, 0.0, "digital silence must floor at 0");
        assert!(quiet > 0.0 && quiet < loud, "quiet {quiet} must sit under loud {loud}");
        assert!(loud <= 1.0, "level must stay normalized, got {loud}");
    }

    #[test]
    fn level_ignores_empty_callbacks() {
        let level = Arc::new(AtomicU32::new(0.5f32.to_bits()));
        store_level::<f32>(&[], &level);
        assert_eq!(read(&level), 0.5, "an empty callback must not reset the level");
    }
}
