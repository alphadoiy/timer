use std::{fs, path::PathBuf};

use serde::Deserialize;

use super::RepeatMode;

#[derive(Debug, Clone)]
pub struct MusicCliOptions {
    pub shuffle: bool,
    pub repeat_mode: Option<RepeatMode>,
    pub volume: Option<u8>,
    pub auto_play: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MusicConfig {
    #[serde(default = "default_shuffle")]
    pub shuffle: bool,
    #[serde(default)]
    pub repeat_mode: RepeatMode,
    #[serde(default = "default_volume")]
    pub volume: u8,
    #[serde(default)]
    pub auto_play: bool,
}

impl Default for MusicConfig {
    fn default() -> Self {
        Self {
            shuffle: default_shuffle(),
            repeat_mode: RepeatMode::Off,
            volume: default_volume(),
            auto_play: false,
        }
    }
}

impl MusicConfig {
    pub fn load() -> Self {
        let path = config_path();
        let Ok(raw) = fs::read_to_string(path) else {
            return Self::default();
        };
        toml::from_str(&raw).unwrap_or_default()
    }

    pub fn merge_cli(mut self, cli: &MusicCliOptions) -> Self {
        if cli.shuffle {
            self.shuffle = true;
        }
        if let Some(repeat_mode) = cli.repeat_mode {
            self.repeat_mode = repeat_mode;
        }
        if let Some(volume) = cli.volume {
            self.volume = volume.min(100);
        }
        if cli.auto_play {
            self.auto_play = true;
        }
        self
    }
}

fn config_path() -> PathBuf {
    let mut base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.push("timer");
    base.push("music.toml");
    base
}

const fn default_shuffle() -> bool {
    false
}

const fn default_volume() -> u8 {
    80
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_overrides_config_values() {
        let cfg = MusicConfig {
            shuffle: false,
            repeat_mode: RepeatMode::Off,
            volume: 12,
            auto_play: false,
        };
        let cli = MusicCliOptions {
            shuffle: true,
            repeat_mode: Some(RepeatMode::All),
            volume: Some(64),
            auto_play: true,
        };

        let merged = cfg.merge_cli(&cli);
        assert!(merged.shuffle);
        assert_eq!(merged.repeat_mode, RepeatMode::All);
        assert_eq!(merged.volume, 64);
        assert!(merged.auto_play);
    }
}
