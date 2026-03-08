use clap::{Parser, Subcommand};

use crate::app::ModeKind;

#[derive(Debug, Clone, Parser)]
#[command(name = "timer", version, about = "Analog clock and pomodoro TUI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
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
}
