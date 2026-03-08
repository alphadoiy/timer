use std::time::Duration;

use serde::{Deserialize, Serialize};

pub mod config;
pub mod engine;
pub mod library;
pub mod provider;
pub mod queue;
pub mod ui;
pub mod visualizer;

pub use config::{MusicCliOptions, MusicConfig};
pub use engine::MusicEngine;
pub use queue::TrackQueue;
pub use visualizer::NUM_BANDS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    Off,
    All,
    One,
}

impl Default for RepeatMode {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    LocalFile,
    HttpStream,
}

#[derive(Debug, Clone)]
pub struct TrackMeta {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub duration: Option<Duration>,
    pub source_kind: SourceKind,
    pub path_or_url: String,
}

#[derive(Debug, Clone)]
pub enum InputRef {
    Path(String),
    Url(String),
}

#[derive(Debug, Clone)]
pub enum MusicCommand {
    Play,
    Pause,
    Toggle,
    Stop,
    Next,
    Prev,
    Seek(i64),
    SetVolume(u8),
    ToggleShuffle,
    SetRepeat(RepeatMode),
    Load(Vec<InputRef>),
}

#[derive(Debug, Clone)]
pub enum PlaybackState {
    Idle,
    Buffering,
    Playing,
    Paused,
    Stopped,
    Ended,
    Error(String),
}

impl PlaybackState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Idle => "IDLE",
            Self::Buffering => "BUFFERING",
            Self::Playing => "PLAYING",
            Self::Paused => "PAUSED",
            Self::Stopped => "STOPPED",
            Self::Ended => "ENDED",
            Self::Error(_) => "ERROR",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizerMode {
    Bars,
    Bricks,
    Columns,
    Wave,
    Scatter,
    Flame,
    Pulse,
    Retro,
    Matrix,
    Binary,
    Snow,
}

impl VisualizerMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bars => "Bars",
            Self::Bricks => "Bricks",
            Self::Columns => "Columns",
            Self::Wave => "Wave",
            Self::Scatter => "Scatter",
            Self::Flame => "Flame",
            Self::Pulse => "Pulse",
            Self::Retro => "Retro",
            Self::Matrix => "Matrix",
            Self::Binary => "Binary",
            Self::Snow => "Snow",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Bars => Self::Bricks,
            Self::Bricks => Self::Columns,
            Self::Columns => Self::Wave,
            Self::Wave => Self::Scatter,
            Self::Scatter => Self::Flame,
            Self::Flame => Self::Retro,
            Self::Retro => Self::Pulse,
            Self::Pulse => Self::Matrix,
            Self::Matrix => Self::Binary,
            Self::Binary => Self::Snow,
            Self::Snow => Self::Bars,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MusicSnapshot {
    pub state: PlaybackState,
    pub queue: Vec<TrackMeta>,
    pub current_index: Option<usize>,
    pub selected_index: usize,
    pub shuffle: bool,
    pub repeat_mode: RepeatMode,
    pub volume: u8,
    pub muted: bool,
    pub visualizer_mode: VisualizerMode,
    pub position: Duration,
    pub duration: Option<Duration>,
    pub spectrum_bands: [f32; NUM_BANDS],
    pub wave_samples: Vec<f32>,
    pub visualizer_frame: u64,
    pub last_error: Option<String>,
}

impl Default for MusicSnapshot {
    fn default() -> Self {
        Self {
            state: PlaybackState::Idle,
            queue: Vec::new(),
            current_index: None,
            selected_index: 0,
            shuffle: false,
            repeat_mode: RepeatMode::Off,
            volume: 80,
            muted: false,
            visualizer_mode: VisualizerMode::Scatter,
            position: Duration::ZERO,
            duration: None,
            spectrum_bands: [0.0; NUM_BANDS],
            wave_samples: Vec::new(),
            visualizer_frame: 0,
            last_error: None,
        }
    }
}
