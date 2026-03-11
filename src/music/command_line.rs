use std::time::{Duration, Instant};

use crossterm::event::KeyCode;

/// Result of executing a command – the `App` dispatches these.
#[derive(Debug, Clone)]
pub enum CommandAction {
    None,
    AddUrl(String),
    LoadUrl(String),
    LoadRadio,
    ClearQueue,
    SetVolume(u8),
    Seek(i64),
    StationAdd { name: String, url: String },
    StationRemove(String),
    StationList,
    ShowSources,
    ShowHelp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmdMode {
    Inactive,
    Input,
    Message,
}

pub struct CommandLine {
    mode: CmdMode,
    buffer: String,
    cursor: usize,
    message: String,
    message_until: Option<Instant>,
    history: Vec<String>,
    history_index: Option<usize>,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            mode: CmdMode::Inactive,
            buffer: String::new(),
            cursor: 0,
            message: String::new(),
            message_until: None,
            history: Vec::new(),
            history_index: None,
        }
    }

    /// Whether the command line is capturing input (should steal all key events).
    pub fn is_active(&self) -> bool {
        self.mode == CmdMode::Input
    }

    /// Whether there is a message to display (input or feedback).
    pub fn is_visible(&self) -> bool {
        self.mode != CmdMode::Inactive
    }

    /// The display string for the status bar.
    pub fn display(&self) -> (&str, usize) {
        match self.mode {
            CmdMode::Input => (&self.buffer, self.cursor),
            CmdMode::Message => (&self.message, 0),
            CmdMode::Inactive => ("", 0),
        }
    }

    /// Is this showing an input prompt (with `:` prefix)?
    pub fn is_input(&self) -> bool {
        self.mode == CmdMode::Input
    }

    /// Activate the command line (called when `:` is pressed).
    pub fn activate(&mut self) {
        self.mode = CmdMode::Input;
        self.buffer.clear();
        self.cursor = 0;
        self.history_index = None;
    }

    /// Dismiss the command line.
    pub fn dismiss(&mut self) {
        self.mode = CmdMode::Inactive;
        self.buffer.clear();
        self.cursor = 0;
        self.history_index = None;
    }

    /// Show a feedback message for a short duration.
    pub fn show_message(&mut self, msg: impl Into<String>) {
        self.message = msg.into();
        self.message_until = Some(Instant::now() + Duration::from_secs(3));
        self.mode = CmdMode::Message;
    }

    /// Call each frame – auto-dismiss timed messages.
    pub fn tick(&mut self) {
        if self.mode == CmdMode::Message {
            if self.message_until.is_some_and(|t| Instant::now() >= t) {
                self.dismiss();
            }
        }
    }

    /// Handle a key event while in Input mode. Returns `Some(action)` on Enter.
    pub fn handle_key(&mut self, code: KeyCode) -> Option<CommandAction> {
        match code {
            KeyCode::Esc => {
                self.dismiss();
                None
            }
            KeyCode::Enter => {
                let input = self.buffer.trim().to_string();
                if !input.is_empty() {
                    self.history.push(input.clone());
                }
                self.mode = CmdMode::Inactive;
                self.buffer.clear();
                self.cursor = 0;
                self.history_index = None;
                if input.is_empty() {
                    return None;
                }
                Some(parse_command(&input))
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let byte_pos = char_to_byte(&self.buffer, self.cursor - 1);
                    let end = char_to_byte(&self.buffer, self.cursor);
                    self.buffer.replace_range(byte_pos..end, "");
                    self.cursor -= 1;
                }
                None
            }
            KeyCode::Delete => {
                let len = self.buffer.chars().count();
                if self.cursor < len {
                    let byte_pos = char_to_byte(&self.buffer, self.cursor);
                    let end = char_to_byte(&self.buffer, self.cursor + 1);
                    self.buffer.replace_range(byte_pos..end, "");
                }
                None
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                None
            }
            KeyCode::Right => {
                let len = self.buffer.chars().count();
                if self.cursor < len {
                    self.cursor += 1;
                }
                None
            }
            KeyCode::Home => {
                self.cursor = 0;
                None
            }
            KeyCode::End => {
                self.cursor = self.buffer.chars().count();
                None
            }
            KeyCode::Up => {
                if !self.history.is_empty() {
                    let idx = match self.history_index {
                        Some(i) => i.saturating_sub(1),
                        None => self.history.len() - 1,
                    };
                    self.history_index = Some(idx);
                    self.buffer = self.history[idx].clone();
                    self.cursor = self.buffer.chars().count();
                }
                None
            }
            KeyCode::Down => {
                if let Some(idx) = self.history_index {
                    if idx + 1 < self.history.len() {
                        let next = idx + 1;
                        self.history_index = Some(next);
                        self.buffer = self.history[next].clone();
                        self.cursor = self.buffer.chars().count();
                    } else {
                        self.history_index = None;
                        self.buffer.clear();
                        self.cursor = 0;
                    }
                }
                None
            }
            KeyCode::Char(ch) => {
                let byte_pos = char_to_byte(&self.buffer, self.cursor);
                self.buffer.insert(byte_pos, ch);
                self.cursor += 1;
                None
            }
            _ => None,
        }
    }
}

fn char_to_byte(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn parse_command(input: &str) -> CommandAction {
    let mut parts = input.splitn(2, ' ');
    let cmd = parts.next().unwrap_or("").to_ascii_lowercase();
    let arg = parts.next().unwrap_or("").trim().to_string();

    match cmd.as_str() {
        "add" | "a" if !arg.is_empty() => CommandAction::AddUrl(arg),
        "load" | "open" | "o" if !arg.is_empty() => CommandAction::LoadUrl(arg),
        "radio" | "r" => CommandAction::LoadRadio,
        "clear" | "c" => CommandAction::ClearQueue,
        "vol" | "volume" | "v" => {
            if let Ok(level) = arg.parse::<u8>() {
                CommandAction::SetVolume(level.min(100))
            } else {
                CommandAction::None
            }
        }
        "seek" | "sk" => {
            let trimmed = arg.trim_start_matches('+');
            if let Ok(secs) = trimmed.parse::<i64>() {
                CommandAction::Seek(secs)
            } else {
                CommandAction::None
            }
        }
        "station" | "st" => parse_station_subcommand(&arg),
        "sources" | "src" => CommandAction::ShowSources,
        "help" | "h" | "?" => CommandAction::ShowHelp,
        _ => CommandAction::None,
    }
}

fn parse_station_subcommand(arg: &str) -> CommandAction {
    let mut parts = arg.splitn(2, ' ');
    let sub = parts.next().unwrap_or("").to_ascii_lowercase();
    let rest = parts.next().unwrap_or("").trim();

    match sub.as_str() {
        "add" | "a" => {
            // Format: station add <name> <url>
            if let Some((name, url)) = rest.rsplit_once(' ') {
                let name = name.trim();
                let url = url.trim();
                if !name.is_empty() && !url.is_empty() {
                    return CommandAction::StationAdd {
                        name: name.to_string(),
                        url: url.to_string(),
                    };
                }
            }
            CommandAction::None
        }
        "rm" | "remove" | "del" | "delete" => {
            if rest.is_empty() {
                CommandAction::None
            } else {
                CommandAction::StationRemove(rest.to_string())
            }
        }
        "list" | "ls" | "" => CommandAction::StationList,
        _ => CommandAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_add_command() {
        let action = parse_command("add https://example.com/stream");
        assert!(matches!(action, CommandAction::AddUrl(url) if url == "https://example.com/stream"));
    }

    #[test]
    fn parse_add_shorthand() {
        let action = parse_command("a https://example.com/stream");
        assert!(matches!(action, CommandAction::AddUrl(_)));
    }

    #[test]
    fn parse_load_command() {
        let action = parse_command("load https://example.com/podcast.xml");
        assert!(matches!(action, CommandAction::LoadUrl(url) if url == "https://example.com/podcast.xml"));
    }

    #[test]
    fn parse_radio_command() {
        let action = parse_command("radio");
        assert!(matches!(action, CommandAction::LoadRadio));
    }

    #[test]
    fn parse_clear_command() {
        let action = parse_command("clear");
        assert!(matches!(action, CommandAction::ClearQueue));
    }

    #[test]
    fn parse_vol_command() {
        let action = parse_command("vol 75");
        assert!(matches!(action, CommandAction::SetVolume(75)));
    }

    #[test]
    fn parse_vol_clamps_to_100() {
        let action = parse_command("vol 200");
        assert!(matches!(action, CommandAction::SetVolume(100)));
    }

    #[test]
    fn parse_seek_forward() {
        let action = parse_command("seek +30");
        assert!(matches!(action, CommandAction::Seek(30)));
    }

    #[test]
    fn parse_seek_backward() {
        let action = parse_command("seek -10");
        assert!(matches!(action, CommandAction::Seek(-10)));
    }

    #[test]
    fn parse_help() {
        let action = parse_command("help");
        assert!(matches!(action, CommandAction::ShowHelp));
    }

    #[test]
    fn parse_unknown_returns_none() {
        let action = parse_command("foobar");
        assert!(matches!(action, CommandAction::None));
    }

    #[test]
    fn parse_sources_command() {
        let action = parse_command("sources");
        assert!(matches!(action, CommandAction::ShowSources));
    }

    #[test]
    fn parse_station_add() {
        let action = parse_command("station add Jazz FM https://jazz.example.com/stream");
        match action {
            CommandAction::StationAdd { name, url } => {
                assert_eq!(name, "Jazz FM");
                assert_eq!(url, "https://jazz.example.com/stream");
            }
            other => panic!("expected StationAdd, got {other:?}"),
        }
    }

    #[test]
    fn parse_station_rm() {
        let action = parse_command("station rm Jazz FM");
        assert!(matches!(action, CommandAction::StationRemove(name) if name == "Jazz FM"));
    }

    #[test]
    fn parse_station_list() {
        let action = parse_command("station list");
        assert!(matches!(action, CommandAction::StationList));
    }

    #[test]
    fn parse_station_shorthand() {
        let action = parse_command("st ls");
        assert!(matches!(action, CommandAction::StationList));
    }

    #[test]
    fn handle_escape_dismisses() {
        let mut cl = CommandLine::new();
        cl.activate();
        assert!(cl.is_active());
        cl.handle_key(KeyCode::Esc);
        assert!(!cl.is_active());
    }

    #[test]
    fn handle_char_inserts_at_cursor() {
        let mut cl = CommandLine::new();
        cl.activate();
        cl.handle_key(KeyCode::Char('a'));
        cl.handle_key(KeyCode::Char('b'));
        cl.handle_key(KeyCode::Char('c'));
        assert_eq!(cl.buffer, "abc");
        assert_eq!(cl.cursor, 3);
    }

    #[test]
    fn handle_backspace_deletes_before_cursor() {
        let mut cl = CommandLine::new();
        cl.activate();
        cl.handle_key(KeyCode::Char('a'));
        cl.handle_key(KeyCode::Char('b'));
        cl.handle_key(KeyCode::Backspace);
        assert_eq!(cl.buffer, "a");
        assert_eq!(cl.cursor, 1);
    }

    #[test]
    fn handle_left_right_moves_cursor() {
        let mut cl = CommandLine::new();
        cl.activate();
        cl.handle_key(KeyCode::Char('a'));
        cl.handle_key(KeyCode::Char('b'));
        cl.handle_key(KeyCode::Left);
        assert_eq!(cl.cursor, 1);
        cl.handle_key(KeyCode::Char('x'));
        assert_eq!(cl.buffer, "axb");
    }

    #[test]
    fn enter_returns_parsed_action() {
        let mut cl = CommandLine::new();
        cl.activate();
        for ch in "add https://x.com".chars() {
            cl.handle_key(KeyCode::Char(ch));
        }
        let action = cl.handle_key(KeyCode::Enter);
        assert!(matches!(action, Some(CommandAction::AddUrl(_))));
        assert!(!cl.is_active());
    }

    #[test]
    fn history_navigation() {
        let mut cl = CommandLine::new();

        // First command
        cl.activate();
        for ch in "add https://a.com".chars() {
            cl.handle_key(KeyCode::Char(ch));
        }
        cl.handle_key(KeyCode::Enter);

        // Second command
        cl.activate();
        for ch in "vol 50".chars() {
            cl.handle_key(KeyCode::Char(ch));
        }
        cl.handle_key(KeyCode::Enter);

        // Navigate history
        cl.activate();
        cl.handle_key(KeyCode::Up);
        assert_eq!(cl.buffer, "vol 50");
        cl.handle_key(KeyCode::Up);
        assert_eq!(cl.buffer, "add https://a.com");
        cl.handle_key(KeyCode::Down);
        assert_eq!(cl.buffer, "vol 50");
    }
}
