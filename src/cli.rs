use clap::{Parser, Subcommand};

use crate::{
    app::ModeKind,
    music::{MusicCliOptions, RepeatMode},
};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "timer",
    version,
    about = "Analog clock, pomodoro and music TUI"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(long, default_value_t = true)]
    pub auto_location: bool,

    #[arg(long)]
    pub lat: Option<f64>,

    #[arg(long)]
    pub lon: Option<f64>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Clock,
    Pomodoro {
        #[arg(long, help = "Use colors suited for light terminal backgrounds")]
        light_bg: bool,
    },
    Music {
        #[arg(value_name = "PATH_OR_URL")]
        inputs: Vec<String>,

        #[arg(long)]
        shuffle: bool,

        #[arg(long, value_enum)]
        repeat: Option<RepeatMode>,

        #[arg(long, value_parser = clap::value_parser!(u8).range(0..=100))]
        volume: Option<u8>,

        #[arg(long)]
        auto_play: bool,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    pub fn initial_mode(&self) -> ModeKind {
        match self.command {
            Some(Command::Clock) => ModeKind::Clock,
            Some(Command::Pomodoro { .. }) => ModeKind::Pomodoro,
            Some(Command::Music { .. }) => ModeKind::Music,
            None => ModeKind::Clock,
        }
    }

    pub fn weather_coords(&self) -> Option<(f64, f64)> {
        match (self.lat, self.lon) {
            (Some(lat), Some(lon)) => Some((lat, lon)),
            _ => None,
        }
    }

    pub fn light_bg(&self) -> bool {
        matches!(&self.command, Some(Command::Pomodoro { light_bg: true }))
    }

    pub fn music_inputs(&self) -> &[String] {
        match &self.command {
            Some(Command::Music { inputs, .. }) => inputs,
            _ => &[],
        }
    }

    pub fn music_options(&self) -> MusicCliOptions {
        match &self.command {
            Some(Command::Music {
                shuffle,
                repeat,
                volume,
                auto_play,
                ..
            }) => MusicCliOptions {
                shuffle: *shuffle,
                repeat_mode: *repeat,
                volume: *volume,
                auto_play: *auto_play,
            },
            _ => MusicCliOptions {
                shuffle: false,
                repeat_mode: None,
                volume: None,
                auto_play: false,
            },
        }
    }
}
