use clap::{Parser, Subcommand};

use crate::app::ModeKind;

#[derive(Debug, Clone, Parser)]
#[command(name = "timer", version, about = "Analog clock and pomodoro TUI")]
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
    Pomodoro,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    pub fn initial_mode(&self) -> ModeKind {
        match self.command {
            Some(Command::Clock) => ModeKind::Clock,
            Some(Command::Pomodoro) => ModeKind::Pomodoro,
            None => ModeKind::Clock,
        }
    }

    pub fn weather_coords(&self) -> Option<(f64, f64)> {
        match (self.lat, self.lon) {
            (Some(lat), Some(lon)) => Some((lat, lon)),
            _ => None,
        }
    }
}
