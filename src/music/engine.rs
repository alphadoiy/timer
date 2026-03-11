use std::{
    io::BufReader,
    io::{Read, Seek, SeekFrom},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use symphonia::{
    core::{
        formats::FormatOptions,
        io::{MediaSource, MediaSourceStream},
        meta::MetadataOptions,
        probe::Hint,
    },
    default::get_probe,
};

use super::{
    MusicCommand, MusicSnapshot, PlaybackState, ProviderKind, RepeatMode, SourceInfo, TrackMeta,
    VisualizerMode,
    provider::open_reader,
    queue::TrackQueue,
    visualizer::{AudioTap, NUM_BANDS, SpectrumAnalyzer, TapSource},
};

pub struct MusicEngine {
    queue: TrackQueue,
    state: PlaybackState,
    volume: u8,
    muted: bool,
    visualizer_mode: VisualizerMode,
    selected_index: usize,
    output_stream: Option<OutputStream>,
    output_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    started_at: Option<Instant>,
    paused_at: Option<Duration>,
    current_duration: Option<Duration>,
    spectrum_bands: [f32; NUM_BANDS],
    wave_samples: Vec<f32>,
    visualizer_frame: u64,
    sample_rate_hz: f32,
    pending_seek_delta_sec: i64,
    pending_seek_requested_at: Option<Instant>,
    last_seek_applied_at: Option<Instant>,
    tap: Arc<Mutex<AudioTap>>,
    analyzer: SpectrumAnalyzer,
    last_error: Option<String>,
}

impl MusicEngine {
    pub fn new(queue: TrackQueue, volume: u8) -> Self {
        let mut engine = Self {
            queue,
            state: PlaybackState::Idle,
            volume: volume.min(100),
            muted: false,
            visualizer_mode: VisualizerMode::Scatter,
            selected_index: 0,
            output_stream: None,
            output_handle: None,
            sink: None,
            started_at: None,
            paused_at: None,
            current_duration: None,
            spectrum_bands: [0.0; NUM_BANDS],
            wave_samples: Vec::new(),
            visualizer_frame: 0,
            sample_rate_hz: 44_100.0,
            pending_seek_delta_sec: 0,
            pending_seek_requested_at: None,
            last_seek_applied_at: None,
            tap: Arc::new(Mutex::new(AudioTap::new(8192))),
            analyzer: SpectrumAnalyzer::new(),
            last_error: None,
        };
        if let Err(err) = engine.ensure_output() {
            engine.state = PlaybackState::Error(err.to_string());
            engine.last_error = Some(err.to_string());
        }
        engine
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn queue_len(&self) -> usize {
        self.queue.tracks().len()
    }

    pub fn select_and_play(&mut self, index: usize) {
        self.selected_index = index.min(self.queue_len().saturating_sub(1));
        if let Err(err) = self.play_at_index(self.selected_index, Duration::ZERO) {
            self.state = PlaybackState::Error(err.to_string());
            self.last_error = Some(err.to_string());
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        let len = self.queue.tracks().len();
        if len == 0 {
            self.selected_index = 0;
            return;
        }
        let next = if delta.is_negative() {
            self.selected_index
                .saturating_sub(delta.unsigned_abs() as usize)
        } else {
            self.selected_index
                .saturating_add(delta as usize)
                .min(len.saturating_sub(1))
        };
        self.selected_index = next;
    }

    pub fn dispatch(&mut self, command: MusicCommand) {
        let result = match command {
            MusicCommand::Play => self.play(),
            MusicCommand::Pause => self.pause(),
            MusicCommand::Toggle => self.toggle(),
            MusicCommand::Stop => self.stop(),
            MusicCommand::Next => self.next(true),
            MusicCommand::Prev => self.prev(),
            MusicCommand::Seek(delta) => {
                self.enqueue_seek(delta);
                Ok(())
            }
            MusicCommand::SetVolume(volume) => {
                self.set_volume(volume);
                Ok(())
            }
            MusicCommand::ToggleShuffle => {
                self.queue.toggle_shuffle();
                Ok(())
            }
            MusicCommand::SetRepeat(mode) => {
                self.queue.set_repeat_mode(mode);
                Ok(())
            }
            MusicCommand::Load(inputs) => {
                let tracks = super::library::build_tracks(&inputs);
                self.load(tracks);
                Ok(())
            }
            MusicCommand::LoadUrl(url) => {
                self.load_url(&url);
                Ok(())
            }
        };

        if let Err(err) = result {
            self.state = PlaybackState::Error(err.to_string());
            self.last_error = Some(err.to_string());
        }
    }

    pub fn load(&mut self, tracks: Vec<TrackMeta>) {
        self.queue.load(tracks);
        self.selected_index = 0;
        self.state = PlaybackState::Idle;
        self.started_at = None;
        self.paused_at = None;
        self.current_duration = None;
        self.spectrum_bands = [0.0; NUM_BANDS];
        if let Ok(mut tap) = self.tap.lock() {
            tap.clear();
        }
    }

    fn load_url(&mut self, url: &str) {
        let inputs = super::library::parse_inputs(&[url.to_string()]);
        let new_tracks = super::library::build_tracks(&inputs);
        if new_tracks.is_empty() {
            self.last_error = Some(format!("No playable tracks found at: {url}"));
            return;
        }
        self.queue.append(new_tracks);
        self.last_error = None;
    }

    pub fn update(&mut self) {
        if matches!(self.state, PlaybackState::Playing) {
            self.visualizer_frame = self.visualizer_frame.wrapping_add(1);
            self.refresh_spectrum();
        }
        self.flush_pending_seek();

        let ended = self
            .sink
            .as_ref()
            .is_some_and(|sink| sink.empty() && matches!(self.state, PlaybackState::Playing));

        if ended {
            self.state = PlaybackState::Ended;
            if self.next(false).is_err() {
                let _ = self.stop();
            }
        }
    }

    pub fn snapshot(&self) -> MusicSnapshot {
        MusicSnapshot {
            state: self.state.clone(),
            queue: self.queue.tracks().to_vec(),
            current_index: self.queue.current_index(),
            selected_index: self.selected_index,
            shuffle: self.queue.shuffle(),
            repeat_mode: self.queue.repeat_mode(),
            volume: self.volume,
            muted: self.muted,
            visualizer_mode: self.visualizer_mode,
            position: self.position(),
            duration: self.current_duration,
            spectrum_bands: self.spectrum_bands,
            wave_samples: self.wave_samples.clone(),
            visualizer_frame: self.visualizer_frame,
            last_error: self.last_error.clone(),
            sources: self.source_summary(),
        }
    }

    pub fn shutdown(&mut self) {
        let _ = self.stop();
    }

    fn play(&mut self) -> Result<()> {
        match self.state {
            PlaybackState::Paused => {
                if let Some(sink) = &self.sink {
                    sink.play();
                    let paused = self.paused_at.unwrap_or(Duration::ZERO);
                    self.started_at = Some(Instant::now() - paused);
                    self.state = PlaybackState::Playing;
                }
                Ok(())
            }
            PlaybackState::Playing => Ok(()),
            _ => {
                let idx = self.queue.current_index().unwrap_or(self.selected_index);
                self.play_at_index(idx, Duration::ZERO)
            }
        }
    }

    fn pause(&mut self) -> Result<()> {
        if let Some(sink) = &self.sink {
            sink.pause();
            self.paused_at = Some(self.position());
            self.state = PlaybackState::Paused;
        }
        Ok(())
    }

    fn toggle(&mut self) -> Result<()> {
        if matches!(self.state, PlaybackState::Playing) {
            self.pause()
        } else {
            self.play()
        }
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.started_at = None;
        self.paused_at = Some(Duration::ZERO);
        self.current_duration = None;
        self.spectrum_bands = [0.0; NUM_BANDS];
        self.wave_samples.clear();
        self.state = PlaybackState::Stopped;
        Ok(())
    }

    fn next(&mut self, manual: bool) -> Result<()> {
        if let Some(track) = self.queue.next(manual).cloned() {
            self.play_track(track, Duration::ZERO, None)
        } else {
            self.state = PlaybackState::Ended;
            Ok(())
        }
    }

    fn prev(&mut self) -> Result<()> {
        if let Some(track) = self.queue.prev().cloned() {
            self.play_track(track, Duration::ZERO, None)
        } else {
            Ok(())
        }
    }

    fn seek(&mut self, delta_sec: i64) -> Result<()> {
        let Some(current) = self.queue.current().cloned() else {
            return Ok(());
        };
        // Disable seeking for streaming providers without known duration
        if !matches!(current.provider, ProviderKind::Local) && self.current_duration.is_none() {
            return Ok(());
        }

        let current_pos = self.position();
        let shifted = if delta_sec.is_negative() {
            current_pos.saturating_sub(Duration::from_secs(delta_sec.unsigned_abs()))
        } else {
            current_pos.saturating_add(Duration::from_secs(delta_sec as u64))
        };

        let known_duration = self.current_duration.or(current.duration);
        self.play_track(current, shifted, known_duration)
    }

    fn enqueue_seek(&mut self, delta_sec: i64) {
        self.pending_seek_delta_sec = self.pending_seek_delta_sec.saturating_add(delta_sec);
        self.pending_seek_requested_at = Some(Instant::now());
    }

    fn flush_pending_seek(&mut self) {
        const SEEK_DEBOUNCE: Duration = Duration::from_millis(120);
        const SEEK_COOLDOWN: Duration = Duration::from_millis(180);

        if self.pending_seek_delta_sec == 0 {
            return;
        }
        let now = Instant::now();
        if self
            .pending_seek_requested_at
            .is_some_and(|t| now.saturating_duration_since(t) < SEEK_DEBOUNCE)
        {
            return;
        }
        if self
            .last_seek_applied_at
            .is_some_and(|t| now.saturating_duration_since(t) < SEEK_COOLDOWN)
        {
            return;
        }

        let delta = std::mem::take(&mut self.pending_seek_delta_sec);
        self.pending_seek_requested_at = None;
        if let Err(err) = self.seek(delta) {
            self.state = PlaybackState::Error(err.to_string());
            self.last_error = Some(err.to_string());
            return;
        }
        self.last_seek_applied_at = Some(now);
    }

    fn play_at_index(&mut self, idx: usize, start_at: Duration) -> Result<()> {
        let Some(track) = self.queue.select(idx).cloned() else {
            return Ok(());
        };
        self.play_track(track, start_at, None)
    }

    fn play_track(
        &mut self,
        track: TrackMeta,
        start_at: Duration,
        known_duration_hint: Option<Duration>,
    ) -> Result<()> {
        self.ensure_output()?;
        self.state = PlaybackState::Buffering;

        if let Some(old_sink) = self.sink.take() {
            old_sink.stop();
        }

        let handle = self
            .output_handle
            .as_ref()
            .context("audio output handle is missing")?;

        let sink = Sink::try_new(handle).context("failed to create audio sink")?;
        let reader = open_reader(&track)?;
        let decoder = Decoder::new(BufReader::new(reader)).context("failed to decode track")?;
        self.sample_rate_hz = decoder.sample_rate() as f32;
        let channels = decoder.channels();
        let total = decoder
            .total_duration()
            .or(known_duration_hint)
            .or(track.duration)
            .or_else(|| Self::probe_duration_fallback(&track));
        let tapped = TapSource::new(decoder.convert_samples(), Arc::clone(&self.tap), channels);

        if start_at > Duration::ZERO {
            sink.append(tapped.skip_duration(start_at));
        } else {
            sink.append(tapped);
        }

        sink.set_volume(self.effective_volume());
        self.current_duration = total;
        if let Some(duration) = total {
            self.queue.set_current_duration(duration);
        }
        self.started_at = Some(Instant::now() - start_at);
        self.paused_at = Some(start_at);
        self.state = PlaybackState::Playing;
        self.sink = Some(sink);
        self.last_error = None;
        Ok(())
    }

    fn probe_duration_fallback(track: &TrackMeta) -> Option<Duration> {
        let reader = open_reader(track).ok()?;
        let mut hint = Hint::new();
        if let Some(ext) = std::path::Path::new(&track.path_or_url)
            .extension()
            .and_then(|it| it.to_str())
        {
            hint.with_extension(ext);
        }

        let mss = MediaSourceStream::new(
            Box::new(SeekableMediaSource { inner: reader }),
            Default::default(),
        );
        let probed = get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .ok()?;
        let format = probed.format;
        let track = format.default_track()?;
        let params = &track.codec_params;
        let (frames, sample_rate) = (params.n_frames?, params.sample_rate?);
        Some(Duration::from_secs_f64(frames as f64 / sample_rate as f64))
    }

    fn set_volume(&mut self, volume: u8) {
        self.volume = volume.min(100);
        if let Some(sink) = &self.sink {
            sink.set_volume(self.effective_volume());
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if let Some(sink) = &self.sink {
            sink.set_volume(self.effective_volume());
        }
    }

    pub fn cycle_visualizer_mode(&mut self) {
        self.visualizer_mode = self.visualizer_mode.next();
    }

    fn effective_volume(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.volume as f32 / 100.0
        }
    }

    fn ensure_output(&mut self) -> Result<()> {
        if self.output_stream.is_some() && self.output_handle.is_some() {
            return Ok(());
        }

        let (stream, handle) =
            OutputStream::try_default().context("failed to open audio output")?;
        self.output_stream = Some(stream);
        self.output_handle = Some(handle);
        Ok(())
    }

    fn position(&self) -> Duration {
        if matches!(self.state, PlaybackState::Paused) {
            return self.paused_at.unwrap_or(Duration::ZERO);
        }
        self.started_at
            .map(|started| Instant::now().saturating_duration_since(started))
            .unwrap_or(Duration::ZERO)
    }

    fn refresh_spectrum(&mut self) {
        let samples = self
            .tap
            .lock()
            .map(|tap| tap.snapshot_latest(2048))
            .unwrap_or_default();
        self.spectrum_bands = self.analyzer.analyze(&samples, self.sample_rate_hz);
        self.wave_samples = samples;
    }

    fn source_summary(&self) -> Vec<SourceInfo> {
        let mut counts = std::collections::HashMap::new();
        for track in self.queue.tracks() {
            *counts.entry(track.provider).or_insert(0) += 1;
        }
        let mut summary: Vec<SourceInfo> = counts
            .into_iter()
            .map(|(kind, count)| SourceInfo { kind, count })
            .collect();
        // Sort by kind name for consistent ordering
        summary.sort_by_key(|info| info.kind.label());
        summary
    }
}

impl Default for MusicEngine {
    fn default() -> Self {
        Self::new(TrackQueue::new(false, RepeatMode::Off), 80)
    }
}

struct SeekableMediaSource {
    inner: Box<dyn super::provider::ReadSeek>,
}

impl Read for SeekableMediaSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Seek for SeekableMediaSource {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl MediaSource for SeekableMediaSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_summary() {
        let mut queue = TrackQueue::new(false, RepeatMode::Off);
        let tracks = vec![
            TrackMeta {
                id: 1,
                title: "Local 1".into(),
                artist: "A".into(),
                duration: None,
                provider: ProviderKind::Local,
                path_or_url: "/tmp/1".into(),
            },
            TrackMeta {
                id: 2,
                title: "Local 2".into(),
                artist: "B".into(),
                duration: None,
                provider: ProviderKind::Local,
                path_or_url: "/tmp/2".into(),
            },
            TrackMeta {
                id: 3,
                title: "Radio 1".into(),
                artist: "C".into(),
                duration: None,
                provider: ProviderKind::Radio,
                path_or_url: "http://radio".into(),
            },
        ];
        queue.load(tracks);
        let engine = MusicEngine::new(queue, 80);
        let summary = engine.source_summary();

        assert_eq!(summary.len(), 2);
        
        // They should be sorted by label: "Local", then "Radio"
        assert_eq!(summary[0].kind, ProviderKind::Local);
        assert_eq!(summary[0].count, 2);
        
        assert_eq!(summary[1].kind, ProviderKind::Radio);
        assert_eq!(summary[1].count, 1);
    }
}
