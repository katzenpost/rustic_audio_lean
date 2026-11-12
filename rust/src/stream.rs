// SPDX-FileCopyrightText: Copyright (C) 2026 David Stainton
// SPDX-License-Identifier: AGPL-3.0-only

//! Raw samples to the speaker and from the microphone, mono and at whatever rate the caller
//! asks for, converted to and from the rate the device runs at. After hushphone's `sound.rs`.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

/// Linear interpolation from one rate to another, a sample at a time, so that a stream can go
/// through it in pieces. What comes out lags what goes in by one input sample.
pub struct Resampler {
    step: f64,
    /// Where the next output falls, in input samples after `prev`.
    t: f64,
    prev: f32,
}

impl Resampler {
    pub fn new(from: u32, to: u32) -> Resampler {
        Resampler {
            step: from as f64 / to as f64,
            t: 1.0,
            prev: 0.0,
        }
    }

    pub fn push(&mut self, x: f32, mut out: impl FnMut(f32)) {
        while self.t < 1.0 {
            out(self.prev + (x - self.prev) * self.t as f32);
            self.t += self.step;
        }
        self.t -= 1.0;
        self.prev = x;
    }
}

/// `x` at rate `from` at rate `to`.
pub fn resample(x: &[f32], from: u32, to: u32) -> Vec<f32> {
    let mut r = Resampler::new(from, to);
    let mut out = Vec::with_capacity((x.len() as u64 * to as u64 / from as u64) as usize + 1);
    // The last sample again, so that the one-sample lag does not cut it off.
    for &s in x.iter().chain(x.last()) {
        r.push(s, |v| out.push(v));
    }
    out
}

/// What the microphone heard, at the listener's rate, until the listener takes it: at most
/// `most` samples, past which what comes in is dropped rather than grow without bound.
pub struct Heard {
    r: Resampler,
    q: VecDeque<f32>,
    most: usize,
}

impl Heard {
    pub fn new(from: u32, to: u32, most: usize) -> Heard {
        Heard {
            r: Resampler::new(from, to),
            q: VecDeque::new(),
            most,
        }
    }

    pub fn push(&mut self, x: impl Iterator<Item = f32>) {
        let (q, most) = (&mut self.q, self.most);
        for s in x {
            self.r.push(s, |v| {
                if q.len() < most {
                    q.push_back(v)
                }
            });
        }
    }

    /// The next `n` samples, once there are that many.
    pub fn take(&mut self, n: usize) -> Option<Vec<f32>> {
        (self.q.len() >= n).then(|| self.q.drain(..n).collect())
    }
}

/// Plays `x`, mono at `rate`, on the default speaker; returns once it has played.
pub fn play(x: &[f32], rate: u32) -> Result<(), String> {
    let dev = cpal::default_host()
        .default_output_device()
        .ok_or("no speaker")?;
    let cfg = dev.default_output_config().map_err(|e| e.to_string())?;
    let (fs, format) = (cfg.sample_rate(), cfg.sample_format());
    let clip = Arc::new(resample(x, rate, fs));
    let at = Arc::new(AtomicUsize::new(0));
    let cfg = cfg.config();
    let stream = match format {
        SampleFormat::F32 => speak::<f32>(&dev, &cfg, clip.clone(), at.clone()),
        SampleFormat::I16 => speak::<i16>(&dev, &cfg, clip.clone(), at.clone()),
        SampleFormat::U16 => speak::<u16>(&dev, &cfg, clip.clone(), at.clone()),
        SampleFormat::I32 => speak::<i32>(&dev, &cfg, clip.clone(), at.clone()),
        f => return Err(format!("the speaker takes {f} samples")),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    let until = Instant::now()
        + Duration::from_secs_f64(clip.len() as f64 / fs as f64)
        + Duration::from_secs(2);
    while at.load(Ordering::Relaxed) < clip.len() {
        if Instant::now() > until {
            return Err("the speaker stopped".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

fn speak<T: SizedSample + FromSample<f32>>(
    dev: &cpal::Device,
    cfg: &cpal::StreamConfig,
    clip: Arc<Vec<f32>>,
    at: Arc<AtomicUsize>,
) -> Result<cpal::Stream, String> {
    let ch = cfg.channels.max(1) as usize;
    dev.build_output_stream(
        cfg,
        move |data: &mut [T], _| {
            for frame in data.chunks_mut(ch) {
                let v = clip
                    .get(at.fetch_add(1, Ordering::Relaxed))
                    .copied()
                    .unwrap_or(0.0);
                frame.fill(T::from_sample(v.clamp(-1.0, 1.0)));
            }
        },
        |e| eprintln!("rustic_audio_lean: speaker: {e}"),
        None,
    )
    .map_err(|e| e.to_string())
}

/// The default microphone, heard at a rate of the listener's; stops when dropped.
pub struct Listen {
    _stream: cpal::Stream,
    pub heard: Arc<Mutex<Heard>>,
}

impl Listen {
    /// Listens at `rate`, keeping up to 2 s (and at least a frame) for the listener to take.
    pub fn start(rate: u32, frame: usize) -> Result<Listen, String> {
        let dev = cpal::default_host()
            .default_input_device()
            .ok_or("no microphone")?;
        let cfg = dev.default_input_config().map_err(|e| e.to_string())?;
        let (fs, format) = (cfg.sample_rate(), cfg.sample_format());
        let heard = Arc::new(Mutex::new(Heard::new(
            fs,
            rate,
            (rate as usize * 2).max(frame),
        )));
        let cfg = cfg.config();
        let stream = match format {
            SampleFormat::F32 => hear::<f32>(&dev, &cfg, heard.clone()),
            SampleFormat::I16 => hear::<i16>(&dev, &cfg, heard.clone()),
            SampleFormat::U16 => hear::<u16>(&dev, &cfg, heard.clone()),
            SampleFormat::I32 => hear::<i32>(&dev, &cfg, heard.clone()),
            f => return Err(format!("the microphone gives {f} samples")),
        }?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Listen {
            _stream: stream,
            heard,
        })
    }
}

fn hear<T: SizedSample>(
    dev: &cpal::Device,
    cfg: &cpal::StreamConfig,
    heard: Arc<Mutex<Heard>>,
) -> Result<cpal::Stream, String>
where
    f32: FromSample<T>,
{
    let ch = cfg.channels.max(1) as usize;
    dev.build_input_stream(
        cfg,
        move |data: &[T], _| {
            if let Ok(mut h) = heard.lock() {
                h.push(data.chunks(ch).map(|f| f[0].to_sample::<f32>()));
            }
        },
        |e| eprintln!("rustic_audio_lean: microphone: {e}"),
        None,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampling() {
        assert_eq!(resample(&[0.0, 1.0], 1, 2), [0.0, 0.5, 1.0, 1.0]);
        assert_eq!(resample(&[0.0, 1.0, 0.5], 2, 1), [0.0, 0.5]);
        let x: Vec<f32> = (0..480).map(|i| (i as f32 * 0.1).sin()).collect();
        assert_eq!(resample(&x, 48000, 48000), x);
        assert_eq!(resample(&x, 16000, 48000).len(), x.len() * 3);
        assert_eq!(resample(&x, 48000, 16000).len(), x.len() / 3);
        assert_eq!(resample(&x, 44100, 48000).len(), 523);
        assert!(resample(&[], 8000, 48000).is_empty());
    }

    #[test]
    fn a_stream_in_pieces_is_the_whole_resampled() {
        let x: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.03).sin()).collect();
        let mut h = Heard::new(44100, 16000, usize::MAX);
        for c in x.chunks(37) {
            h.push(c.iter().copied());
        }
        let mut got = Vec::new();
        while let Some(f) = h.take(10) {
            got.extend(f);
        }
        let want = resample(&x, 44100, 16000);
        assert_eq!(got[..], want[..got.len()]);
        assert!(want.len() - got.len() < 10);
    }

    #[test]
    fn frames_only_when_ready_and_never_more_than_most() {
        let mut h = Heard::new(8000, 8000, 100);
        h.push([0.5; 4].into_iter());
        assert_eq!(h.take(4), None, "one sample behind");
        h.push([0.5].into_iter());
        assert_eq!(h.take(4), Some(vec![0.5; 4]));
        h.push(std::iter::repeat_n(0.25, 1000));
        assert_eq!(h.q.len(), 100);
        assert!(h.take(100).is_some());
        assert_eq!(h.take(1), None);
    }
}
