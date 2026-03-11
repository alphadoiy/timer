use std::sync::{Arc, Mutex};

use rodio::Source;
use rustfft::{FftPlanner, num_complex::Complex};

pub const NUM_BANDS: usize = 16;
const FFT_SIZE: usize = 2048;
const BAND_EDGES_HZ: [f32; NUM_BANDS + 1] = [
    20.0, 60.0, 100.0, 160.0, 250.0, 400.0, 630.0, 1000.0, 1600.0, 2500.0, 4000.0, 6300.0, 9000.0,
    12000.0, 15000.0, 17500.0, 20000.0,
];

#[derive(Debug)]
pub struct AudioTap {
    ring: Vec<f32>,
    write_idx: usize,
    filled: usize,
}

impl AudioTap {
    pub fn new(capacity: usize) -> Self {
        Self {
            ring: vec![0.0; capacity.max(FFT_SIZE)],
            write_idx: 0,
            filled: 0,
        }
    }

    pub fn push(&mut self, sample: f32) {
        self.ring[self.write_idx] = sample;
        self.write_idx = (self.write_idx + 1) % self.ring.len();
        self.filled = (self.filled + 1).min(self.ring.len());
    }

    pub fn snapshot_latest(&self, len: usize) -> Vec<f32> {
        if self.filled == 0 {
            return Vec::new();
        }
        let take = len.min(self.filled);
        let mut out = Vec::with_capacity(take);
        let start = (self.write_idx + self.ring.len() - take) % self.ring.len();
        for i in 0..take {
            out.push(self.ring[(start + i) % self.ring.len()]);
        }
        out
    }

    pub fn clear(&mut self) {
        self.write_idx = 0;
        self.filled = 0;
        self.ring.fill(0.0);
    }
}

pub struct TapSource<S> {
    inner: S,
    tap: Arc<Mutex<AudioTap>>,
    channels: u16,
    channel_cursor: u16,
}

impl<S> TapSource<S> {
    pub fn new(inner: S, tap: Arc<Mutex<AudioTap>>, channels: u16) -> Self {
        Self {
            inner,
            tap,
            channels: channels.max(1),
            channel_cursor: 0,
        }
    }
}

impl<S> Iterator for TapSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        if self.channel_cursor == 0
            && let Ok(mut tap) = self.tap.lock()
        {
            tap.push(sample);
        }
        self.channel_cursor = (self.channel_cursor + 1) % self.channels;
        Some(sample)
    }
}

impl<S> Source for TapSource<S>
where
    S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}

pub struct SpectrumAnalyzer {
    prev: [f32; NUM_BANDS],
    hann: Vec<f32>,
    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
    bins: Vec<Complex<f32>>,
}

impl SpectrumAnalyzer {
    pub fn new() -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let hann = (0..FFT_SIZE)
            .map(|i| {
                0.5 * (1.0
                    - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE.saturating_sub(1)) as f32)
                        .cos())
            })
            .collect::<Vec<_>>();
        Self {
            prev: [0.0; NUM_BANDS],
            hann,
            fft,
            bins: vec![Complex::new(0.0, 0.0); FFT_SIZE],
        }
    }

    pub fn analyze(&mut self, samples: &[f32], sample_rate: f32) -> [f32; NUM_BANDS] {
        if samples.is_empty() || sample_rate <= 0.0 {
            let mut decayed = [0.0; NUM_BANDS];
            for (i, value) in decayed.iter_mut().enumerate() {
                *value = self.prev[i] * 0.8;
                self.prev[i] = *value;
            }
            return decayed;
        }

        for bin in &mut self.bins {
            *bin = Complex::new(0.0, 0.0);
        }

        let take = FFT_SIZE.min(samples.len());
        let offset = samples.len() - take;
        for i in 0..take {
            self.bins[i].re = samples[offset + i] * self.hann[i];
        }

        self.fft.process(&mut self.bins);

        let mut bands = [0.0; NUM_BANDS];
        let half = FFT_SIZE / 2;
        let bin_hz = sample_rate / FFT_SIZE as f32;

        for b in 0..NUM_BANDS {
            let lo = ((BAND_EDGES_HZ[b] / bin_hz) as usize).clamp(1, half.saturating_sub(1));
            let hi = ((BAND_EDGES_HZ[b + 1] / bin_hz) as usize).clamp(lo, half.saturating_sub(1));
            let mut sum = 0.0;
            let mut count = 0usize;
            for i in lo..=hi {
                sum += self.bins[i].norm();
                count += 1;
            }
            if count > 0 {
                sum /= count as f32;
            }

            let db_norm = ((20.0 * (sum.max(1e-9)).log10() + 10.0) / 50.0).clamp(0.0, 1.0);
            let smoothed = if db_norm > self.prev[b] {
                db_norm * 0.6 + self.prev[b] * 0.4
            } else {
                db_norm * 0.25 + self.prev[b] * 0.75
            };
            bands[b] = smoothed;
            self.prev[b] = smoothed;
        }

        bands
    }
}
