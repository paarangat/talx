use rustfft::{FftPlanner, num_complex::Complex};

pub fn apply(samples: &[i16], sample_rate: u32, semitones: f32) -> Vec<i16> {
    let ratio = 2.0f32.powf(semitones / 12.0);

    let signal: Vec<f32> = samples.iter().map(|&s| s as f32 / 32768.0).collect();

    let n = 2048;
    let hop = n / 4;
    let half_n = n / 2;

    if signal.len() < n {
        return samples.to_vec();
    }

    let num_frames = (signal.len() - n) / hop + 1;

    let window: Vec<f32> = (0..n)
        .map(|i| {
            0.5
                * (1.0
                    - (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos())
        })
        .collect();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);
    let ifft = planner.plan_fft_inverse(n);

    let mut out = vec![0.0f32; signal.len()];
    let mut wsum = vec![0.0f32; signal.len()];
    let mut prev_phase = vec![0.0f32; half_n + 1];
    let mut synth_phase = vec![0.0f32; half_n + 1];

    for frame in 0..num_frames {
        let start = frame * hop;

        let mut buf: Vec<Complex<f32>> = (0..n)
            .map(|i| Complex::new(signal[start + i] * window[i], 0.0))
            .collect();
        fft.process(&mut buf);

        for k in 0..=half_n {
            let mag = buf[k].norm();
            let phase = buf[k].arg();

            let bin_freq = k as f32 * sample_rate as f32 / n as f32;
            let expected =
                2.0 * std::f32::consts::PI * bin_freq * hop as f32 / sample_rate as f32;

            let mut delta = phase - prev_phase[k] - expected;
            delta = delta.rem_euclid(2.0 * std::f32::consts::PI);
            if delta > std::f32::consts::PI {
                delta -= 2.0 * std::f32::consts::PI;
            }

            let true_freq = bin_freq
                + delta * sample_rate as f32
                    / (2.0 * std::f32::consts::PI * hop as f32);
            let new_freq = true_freq * ratio;

            synth_phase[k] +=
                2.0 * std::f32::consts::PI * new_freq * hop as f32 / sample_rate as f32;
            synth_phase[k] = synth_phase[k].rem_euclid(2.0 * std::f32::consts::PI);

            prev_phase[k] = phase;

            buf[k] = Complex::from_polar(mag / n as f32, synth_phase[k]);
            if k > 0 && k < half_n {
                buf[n - k] = buf[k].conj();
            }
        }

        ifft.process(&mut buf);

        for i in 0..n {
            if start + i < out.len() {
                out[start + i] += buf[i].re * window[i];
                wsum[start + i] += window[i];
            }
        }
    }

    for i in 0..out.len() {
        if wsum[i] > 0.0001 {
            out[i] /= wsum[i];
        }
    }

    let peak = out.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
    if peak > 0.0 {
        let s = 1.0 / peak;
        for v in &mut out {
            *v *= s;
        }
    }

    out.iter()
        .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect()
}
