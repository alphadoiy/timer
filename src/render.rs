use std::sync::OnceLock;

use figlet_rs::FIGfont;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    animation::SpritePose,
    app::{ModeKind, SystemStats},
    modes::{clock::ClockSnapshot, pomodoro::PomodoroSnapshot},
    music::{MusicSnapshot, ui as music_ui},
    theme::Theme,
};

mod clock;
mod pomodoro;
mod weathr;

pub struct DashboardView<'a> {
    pub mode: ModeKind,
    pub clock: &'a ClockSnapshot,
    pub pomodoro: PomodoroSnapshot,
    pub music: &'a MusicSnapshot,
    pub music_full_visualizer: bool,
    pub music_queue_overlay: bool,
    pub pose: SpritePose,
    pub theme: Theme,
    pub system: SystemStats,
}

impl Widget for DashboardView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let outer = centered_rect(area, 96, 96);
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.outline))
            .render(outer, buf);

        let inner = Rect {
            x: outer.x + 1,
            y: outer.y + 1,
            width: outer.width.saturating_sub(2),
            height: outer.height.saturating_sub(2),
        };
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(15),
                Constraint::Length(1),
                Constraint::Length(2),
            ])
            .split(inner);

        self.render_header(sections[0], buf);
        self.render_body(sections[1], buf);
        self.render_footer(sections[2], buf);
        self.render_status_bar(sections[3], buf);
    }
}

impl DashboardView<'_> {
    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![
            Span::styled(
                " Braille Dial ",
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "ratatui clock + pomodoro + music",
                Style::default().fg(self.theme.subtext),
            ),
        ]);
        Paragraph::new(title)
            .alignment(Alignment::Center)
            .render(area, buf);
    }

    fn render_body(&self, area: Rect, buf: &mut Buffer) {
        if self.mode == ModeKind::Music {
            if self.music_full_visualizer {
                self.render_music_full_visualizer(area, buf);
            } else {
                self.render_music_body(area, buf);
            }
            if self.music_queue_overlay {
                self.render_music_queue_overlay(area, buf);
            }
            return;
        }

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
            .split(area);

        self.render_visual_panel(cols[0], buf);
        self.render_info_panel(cols[1], buf);
    }

    fn render_visual_panel(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(Line::from(Span::styled(
                match self.mode {
                    ModeKind::Clock => " Analog Clock ",
                    ModeKind::Pomodoro => " Pomodoro Road ",
                    ModeKind::Music => " Music Deck ",
                },
                Style::default()
                    .fg(self.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            )))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.outline));
        let inner = block.inner(area);
        block.render(area, buf);

        match self.mode {
            ModeKind::Clock => {
                clock::render_clock_visual_panel(inner, buf, self.pose, self.theme, self.clock);
            }
            ModeKind::Pomodoro => {
                pomodoro::render_pomodoro_road_panel(
                    inner,
                    buf,
                    self.pomodoro,
                    self.pose,
                    self.theme,
                );
            }
            ModeKind::Music => self.render_music_visual_panel(inner, buf),
        }
    }

    fn render_info_panel(&self, area: Rect, buf: &mut Buffer) {
        if self.mode == ModeKind::Music {
            self.render_music_info_panel(area, buf);
            return;
        }

        let block = Block::default()
            .title(Line::from(Span::styled(
                " Readout ",
                Style::default()
                    .fg(self.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            )))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.outline));
        let inner = block.inner(area);
        block.render(area, buf);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Min(3),
            ])
            .split(inner);

        Paragraph::new(vec![
            Line::from(Span::styled(
                self.clock.time_text.clone(),
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                self.clock.date_text.clone(),
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("System Time", self.theme))
        .alignment(Alignment::Center)
        .render(rows[0], buf);

        Paragraph::new(vec![
            Line::from(Span::styled(
                if self.mode == ModeKind::Clock {
                    "Clock Mode"
                } else {
                    self.pomodoro.phase.label()
                },
                Style::default()
                    .fg(self.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format_duration(self.pomodoro.remaining),
                Style::default().fg(self.theme.text),
            )),
        ])
        .block(box_block("Pomodoro", self.theme))
        .alignment(Alignment::Center)
        .render(rows[1], buf);

        let status = if self.pomodoro.completed {
            "Completed"
        } else if self.pomodoro.running {
            "Running"
        } else {
            "Paused"
        };
        Paragraph::new(vec![
            Line::from(Span::styled(
                format!("Cycle {}", self.pomodoro.cycle),
                Style::default().fg(self.theme.highlight),
            )),
            Line::from(Span::styled(
                status,
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("Session", self.theme))
        .alignment(Alignment::Center)
        .render(rows[2], buf);

        Paragraph::new(vec![
            Line::from(Span::styled(
                progress_bar(self.pomodoro.progress, 20),
                Style::default().fg(self.theme.accent_soft),
            )),
            Line::from(Span::styled(
                "Tab/Arrows switch",
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("Progress", self.theme))
        .alignment(Alignment::Center)
        .render(rows[3], buf);

        Paragraph::new(vec![
            Line::from(Span::styled(
                self.weather_label(),
                Style::default()
                    .fg(self.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Live local weather",
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("Weather", self.theme))
        .alignment(Alignment::Center)
        .render(rows[4], buf);

        Paragraph::new(vec![
            Line::from(Span::styled(
                format!(
                    "{:>5.1}% CPU   {:>4}/{:>4} MiB",
                    self.system.cpu_usage,
                    self.system.memory_used_mib,
                    self.system.memory_total_mib
                ),
                Style::default().fg(self.theme.highlight),
            )),
            Line::from(Span::styled(
                "Live system feed",
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("System", self.theme))
        .alignment(Alignment::Center)
        .render(rows[5], buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let text = match self.mode {
            ModeKind::Clock => {
                "Braille renderer: 2x horizontal and 4x vertical sub-pixels per terminal cell."
            }
            ModeKind::Pomodoro => {
                "Countdown ring shrinks continuously with pseudo-shadowed hands for depth."
            }
            ModeKind::Music => {
                "Built-in player mode: local files + HTTP stream with queue controls."
            }
        };
        Paragraph::new(Line::from(Span::styled(
            text,
            Style::default().fg(self.theme.subtext),
        )))
        .alignment(Alignment::Center)
        .render(area, buf);
    }

    fn render_status_bar(&self, area: Rect, buf: &mut Buffer) {
        let mode = match self.mode {
            ModeKind::Clock => "CLOCK",
            ModeKind::Pomodoro => self.pomodoro.phase.label(),
            ModeKind::Music => "MUSIC",
        };
        let line = if self.mode == ModeKind::Music {
            format!(
                " {mode} | CPU {:>5.1}% | MEM {:>4}/{:>4} MiB | {} | vol {}% | rep {} | shuf {} | vis {} | [Tab] switch [Space] play/pause [n/p] next/prev [↑/↓] select [v] mode [V] fullscreen [Q] queue [q] quit ",
                self.system.cpu_usage,
                self.system.memory_used_mib,
                self.system.memory_total_mib,
                self.music.state.label(),
                self.music.volume,
                music_ui::repeat_label(self.music.repeat_mode),
                if self.music.shuffle { "on" } else { "off" },
                self.music.visualizer_mode.label(),
            )
        } else {
            let status = if self.pomodoro.completed {
                "COMPLETED"
            } else if self.pomodoro.running {
                "RUNNING"
            } else {
                "PAUSED"
            };
            format!(
                " {mode} | CPU {:>5.1}% | MEM {:>4}/{:>4} MiB | {} | [Tab/←/→] switch  [Space] start/pause  [r] reset  [q] quit ",
                self.system.cpu_usage,
                self.system.memory_used_mib,
                self.system.memory_total_mib,
                status
            )
        };
        Paragraph::new(Line::from(Span::styled(
            line,
            Style::default()
                .fg(self.theme.text)
                .bg(self.theme.shadow)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .render(area, buf);
    }

    fn weather_label(&self) -> String {
        pomodoro::current_weather_summary()
    }

    fn render_music_body(&self, area: Rect, buf: &mut Buffer) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(6),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Min(6),
                Constraint::Length(2),
            ])
            .split(area);

        self.render_cliamp_title(rows[0], buf);
        self.render_cliamp_track(rows[1], buf);
        self.render_cliamp_time_status(rows[2], buf);
        self.render_cliamp_visualizer(rows[3], buf);
        self.render_cliamp_seek(rows[4], buf);
        self.render_cliamp_controls(rows[5], buf);
        self.render_cliamp_playlist_header(rows[6], buf);
        self.render_cliamp_playlist(rows[7], buf);
        self.render_cliamp_help(rows[8], buf);
    }

    fn render_cliamp_title(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(Line::from(vec![
            Span::styled(
                "C L I A M P",
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  (timer edition)", Style::default().fg(self.theme.subtext)),
        ]))
        .alignment(Alignment::Left)
        .render(area, buf);
    }

    fn render_cliamp_track(&self, area: Rect, buf: &mut Buffer) {
        let track = self
            .music
            .current_index
            .and_then(|idx| self.music.queue.get(idx))
            .map(|t| t.title.as_str())
            .unwrap_or("No track loaded");
        let artist = self
            .music
            .current_index
            .and_then(|idx| self.music.queue.get(idx))
            .map(|t| t.artist.as_str())
            .unwrap_or("Unknown");

        let line1 = Line::from(vec![
            Span::styled("♫ ", Style::default().fg(Color::LightYellow)),
            Span::styled(track, Style::default().fg(Color::LightYellow)),
        ]);
        let line2 = Line::from(Span::styled(
            format!("  {artist}"),
            Style::default().fg(self.theme.subtext),
        ));
        Paragraph::new(vec![line1, line2]).render(area, buf);
    }

    fn render_cliamp_time_status(&self, area: Rect, buf: &mut Buffer) {
        let time = music_ui::duration_text(self.music.position, self.music.duration);
        let status = match self.music.state {
            crate::music::PlaybackState::Playing => "▶ Playing",
            crate::music::PlaybackState::Paused => "⏸ Paused",
            crate::music::PlaybackState::Buffering => "◌ Buffering",
            crate::music::PlaybackState::Ended => "■ Ended",
            crate::music::PlaybackState::Error(_) => "⚠ Error",
            crate::music::PlaybackState::Idle | crate::music::PlaybackState::Stopped => "■ Stopped",
        };
        let mut text = time.clone();
        let space = area
            .width
            .saturating_sub((time.len() + status.len()) as u16) as usize;
        text.push_str(&" ".repeat(space.max(1)));
        text.push_str(status);
        Paragraph::new(Line::from(Span::styled(
            text,
            Style::default().fg(Color::White),
        )))
        .render(area, buf);
    }

    fn render_cliamp_visualizer(&self, area: Rect, buf: &mut Buffer) {
        let h = area.height as usize;
        let w = area.width as usize;
        if h == 0 || w == 0 {
            return;
        }
        let bands = self.music.spectrum_bands;
        let styled = match self.music.visualizer_mode {
            crate::music::VisualizerMode::Bars => styled_from_plain(render_bars(bands, w, h), h),
            crate::music::VisualizerMode::Bricks => {
                styled_from_plain(render_bricks(bands, w, h), h)
            }
            crate::music::VisualizerMode::Columns => {
                styled_from_plain(render_columns(bands, w, h), h)
            }
            crate::music::VisualizerMode::Wave => {
                styled_from_plain(render_braille_wave(&self.music.wave_samples, w, h), h)
            }
            crate::music::VisualizerMode::Scatter => styled_from_plain(
                render_braille_scatter(bands, w, h, self.music.visualizer_frame),
                h,
            ),
            crate::music::VisualizerMode::Flame => styled_from_plain(
                render_braille_flame(bands, w, h, self.music.visualizer_frame),
                h,
            ),
            crate::music::VisualizerMode::Pulse => {
                render_braille_pulse(bands, w, h, self.music.visualizer_frame)
            }
            crate::music::VisualizerMode::Retro => {
                render_braille_retro(bands, w, h, self.music.visualizer_frame)
            }
            crate::music::VisualizerMode::Matrix => {
                render_matrix_styled(bands, w, h, self.music.visualizer_frame)
            }
            crate::music::VisualizerMode::Binary => {
                render_binary_styled(bands, w, h, self.music.visualizer_frame)
            }
            crate::music::VisualizerMode::Snow => styled_from_plain(
                render_braille_snow(bands, w, h, self.music.visualizer_frame),
                h,
            ),
        };

        for (idx, line) in styled.iter().enumerate() {
            Paragraph::new(styled_line_to_spans(line)).render(
                Rect {
                    x: area.x,
                    y: area.y + idx as u16,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
        }
    }

    fn render_cliamp_seek(&self, area: Rect, buf: &mut Buffer) {
        let progress = if let Some(total) = self.music.duration {
            if total.is_zero() {
                0.0
            } else {
                self.music.position.as_secs_f32() / total.as_secs_f32()
            }
        } else {
            0.0
        }
        .clamp(0.0, 1.0);
        let width = area.width as usize;
        if width == 0 {
            return;
        }
        let filled = (progress * (width.saturating_sub(1)) as f32).round() as usize;
        let mut line = String::with_capacity(width);
        line.push_str(&"━".repeat(filled));
        line.push('●');
        line.push_str(&"━".repeat(width.saturating_sub(filled + 1)));
        Paragraph::new(Line::from(Span::styled(
            line,
            Style::default().fg(Color::LightYellow),
        )))
        .render(area, buf);
    }

    fn render_cliamp_controls(&self, area: Rect, buf: &mut Buffer) {
        let left = format!(
            "EQ [{}]  SHUF:{}  REP:{}",
            self.music.visualizer_mode.label(),
            if self.music.shuffle { "ON" } else { "OFF" },
            music_ui::repeat_label(self.music.repeat_mode)
        );
        let right = format!(
            "VOL {:>3}% {}",
            self.music.volume,
            if self.music.muted { "[M]" } else { "" }
        );
        let gap = area
            .width
            .saturating_sub((left.len() + right.len()) as u16)
            .max(1) as usize;
        Paragraph::new(Line::from(vec![
            Span::styled(left, Style::default().fg(Color::LightCyan)),
            Span::raw(" ".repeat(gap)),
            Span::styled(right, Style::default().fg(Color::Green)),
        ]))
        .render(area, buf);
    }

    fn render_cliamp_playlist_header(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(Line::from(vec![
            Span::styled(
                "PLAYLIST",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {} tracks", self.music.queue.len()),
                Style::default().fg(self.theme.subtext),
            ),
        ]))
        .render(area, buf);
    }

    fn render_cliamp_playlist(&self, area: Rect, buf: &mut Buffer) {
        let lines = music_ui::queue_lines(self.music, area.height as usize);
        let content = lines
            .into_iter()
            .map(|line| {
                let style = if line.contains('▶') {
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD)
                } else if line.starts_with('>') {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(line, style))
            })
            .collect::<Vec<_>>();
        Paragraph::new(content).render(area, buf);
    }

    fn render_cliamp_help(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(Line::from(vec![
            Span::styled("[Space] ", Style::default().fg(self.theme.subtext)),
            Span::styled("⏯ ", Style::default().fg(Color::White)),
            Span::styled("[n/p] ", Style::default().fg(self.theme.subtext)),
            Span::styled("track ", Style::default().fg(Color::White)),
            Span::styled("[v] ", Style::default().fg(self.theme.subtext)),
            Span::styled("vis ", Style::default().fg(Color::White)),
            Span::styled("[V] ", Style::default().fg(self.theme.subtext)),
            Span::styled("full ", Style::default().fg(Color::White)),
            Span::styled("[Q] ", Style::default().fg(self.theme.subtext)),
            Span::styled("queue ", Style::default().fg(Color::White)),
            Span::styled("[m] ", Style::default().fg(self.theme.subtext)),
            Span::styled("mute ", Style::default().fg(Color::White)),
            Span::styled("[q] quit", Style::default().fg(self.theme.subtext)),
        ]))
        .render(area, buf);
    }

    fn render_music_full_visualizer(&self, area: Rect, buf: &mut Buffer) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .split(area);

        let now_playing = self
            .music
            .current_index
            .and_then(|idx| self.music.queue.get(idx))
            .map(|t| t.title.clone())
            .unwrap_or_else(|| "No track loaded".to_string());

        Paragraph::new(vec![
            Line::from(Span::styled(
                now_playing,
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                music_ui::duration_text(self.music.position, self.music.duration),
                Style::default().fg(Color::White),
            )),
        ])
        .alignment(Alignment::Center)
        .render(rows[0], buf);

        self.render_cliamp_seek(rows[1], buf);
        self.render_cliamp_visualizer(rows[2], buf);
        Paragraph::new(Line::from(Span::styled(
            "[V] exit fullscreen  [v] switch mode  [Space] play/pause",
            Style::default().fg(self.theme.subtext),
        )))
        .alignment(Alignment::Center)
        .render(rows[3], buf);
    }

    fn render_music_queue_overlay(&self, area: Rect, buf: &mut Buffer) {
        let overlay = centered_rect(area, 72, 68);
        Block::default()
            .title(Line::from(Span::styled(
                " Queue Manager ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.accent_soft))
            .style(Style::default().bg(self.theme.shadow))
            .render(overlay, buf);
        let inner = Rect {
            x: overlay.x + 1,
            y: overlay.y + 1,
            width: overlay.width.saturating_sub(2),
            height: overlay.height.saturating_sub(2),
        };
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(inner);
        let lines = music_ui::queue_lines(self.music, rows[0].height as usize);
        let content = lines
            .into_iter()
            .map(|line| {
                let style = if line.contains('▶') {
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD)
                } else if line.starts_with('>') {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(line, style))
            })
            .collect::<Vec<_>>();
        Paragraph::new(content).render(rows[0], buf);
        Paragraph::new(Line::from(Span::styled(
            "[Q] close  [↑/↓] select  [Enter] play",
            Style::default().fg(self.theme.subtext),
        )))
        .alignment(Alignment::Center)
        .render(rows[1], buf);
    }

    fn render_music_visual_panel(&self, area: Rect, buf: &mut Buffer) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(5),
                Constraint::Length(2),
            ])
            .split(area);
        let now_playing = self
            .music
            .current_index
            .and_then(|idx| self.music.queue.get(idx))
            .map(|t| t.title.clone())
            .unwrap_or_else(|| "No track loaded".to_string());
        Paragraph::new(vec![
            Line::from(Span::styled(
                "Now Playing",
                Style::default().fg(self.theme.subtext),
            )),
            Line::from(Span::styled(
                now_playing,
                Style::default()
                    .fg(self.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                music_ui::duration_text(self.music.position, self.music.duration),
                Style::default().fg(self.theme.accent),
            )),
        ])
        .block(box_block("Current", self.theme))
        .alignment(Alignment::Center)
        .render(rows[0], buf);

        let queue_lines =
            music_ui::queue_lines(self.music, rows[1].height.saturating_sub(2) as usize);
        let queue_text = queue_lines
            .into_iter()
            .map(|line| Line::from(Span::styled(line, Style::default().fg(self.theme.text))))
            .collect::<Vec<_>>();
        Paragraph::new(queue_text)
            .block(box_block("Queue", self.theme))
            .render(rows[1], buf);

        Paragraph::new(Line::from(Span::styled(
            "Space toggle | n/p next/prev | s shuffle | r repeat | m mute",
            Style::default().fg(self.theme.subtext),
        )))
        .alignment(Alignment::Center)
        .render(rows[2], buf);
    }

    fn render_music_info_panel(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(Line::from(Span::styled(
                " Music Readout ",
                Style::default()
                    .fg(self.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            )))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.outline));
        let inner = block.inner(area);
        block.render(area, buf);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Min(3),
            ])
            .split(inner);

        Paragraph::new(vec![
            Line::from(Span::styled(
                music_ui::mode_label(self.music),
                Style::default().fg(self.theme.highlight),
            )),
            Line::from(Span::styled(
                format!("{} tracks", self.music.queue.len()),
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("Playback", self.theme))
        .alignment(Alignment::Center)
        .render(rows[0], buf);

        Paragraph::new(vec![
            Line::from(Span::styled(
                format!("Volume {}%", self.music.volume),
                Style::default().fg(self.theme.accent),
            )),
            Line::from(Span::styled(
                if self.music.muted {
                    "Muted"
                } else {
                    "Output On"
                },
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("Audio", self.theme))
        .alignment(Alignment::Center)
        .render(rows[1], buf);

        Paragraph::new(vec![
            Line::from(Span::styled(
                format!("Repeat {}", music_ui::repeat_label(self.music.repeat_mode)),
                Style::default().fg(self.theme.highlight),
            )),
            Line::from(Span::styled(
                format!("Shuffle {}", if self.music.shuffle { "On" } else { "Off" }),
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("Queue", self.theme))
        .alignment(Alignment::Center)
        .render(rows[2], buf);

        Paragraph::new(vec![
            Line::from(Span::styled(
                music_ui::duration_text(self.music.position, self.music.duration),
                Style::default().fg(self.theme.text),
            )),
            Line::from(Span::styled(
                "Left/Right seek",
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("Timeline", self.theme))
        .alignment(Alignment::Center)
        .render(rows[3], buf);

        let err = self
            .music
            .last_error
            .clone()
            .unwrap_or_else(|| "No errors".to_string());
        Paragraph::new(Line::from(Span::styled(
            err,
            Style::default().fg(self.theme.subtext),
        )))
        .block(box_block("Health", self.theme))
        .alignment(Alignment::Center)
        .render(rows[4], buf);
    }
}

fn render_figlet_time(area: Rect, buf: &mut Buffer, text: &str, theme: Theme) {
    render_figlet_time_shifted(area, buf, text, theme, 0, 0);
}

fn render_figlet_time_shifted(
    area: Rect,
    buf: &mut Buffer,
    text: &str,
    theme: Theme,
    shift_x: i16,
    shift_y: i16,
) {
    let mut lines = figlet_lines(text, area.width as usize);
    if lines.is_empty() {
        lines.push(text.to_string());
    }

    let mut start_y = area.y as i16 + (area.height.saturating_sub(lines.len() as u16) / 2) as i16;
    start_y += shift_y;
    let shifted = Rect {
        x: area.x.saturating_add_signed(shift_x),
        y: area.y,
        width: area.width,
        height: area.height,
    };
    for (idx, line) in lines.iter().enumerate() {
        let y = start_y + idx as i16;
        if y < area.y as i16 || y >= area.bottom() as i16 {
            break;
        }
        let solid_line = solidify_figlet_line(line);
        put_centered_gradient(
            buf,
            shifted,
            y,
            &solid_line,
            theme.accent,
            theme.highlight,
            theme.danger,
        );
    }
}

fn box_block<'a>(title: &'a str, theme: Theme) -> Block<'a> {
    Block::default()
        .title(Line::from(Span::styled(
            format!(" {} ", title),
            Style::default().fg(theme.accent_soft),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.outline))
}

fn centered_rect(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}

fn progress_bar(progress: f32, width: usize) -> String {
    let filled = (progress.clamp(0.0, 1.0) * width as f32).round() as usize;
    let mut out = String::with_capacity(width);
    for idx in 0..width {
        out.push(if idx < filled { '●' } else { '·' });
    }
    out
}

fn format_duration(duration: std::time::Duration) -> String {
    let total = duration.as_secs();
    let minutes = total / 60;
    let seconds = total % 60;
    format!("{minutes:02}:{seconds:02}")
}

fn truncate_to_char_count(s: &mut String, max_chars: usize) {
    if s.chars().count() <= max_chars {
        return;
    }
    if let Some((byte_idx, _)) = s.char_indices().nth(max_chars) {
        s.truncate(byte_idx);
    }
}

#[derive(Debug, Clone)]
struct StyledVisLine {
    text: String,
    tags: Vec<u8>,
}

fn styled_from_plain(lines: Vec<String>, total_rows: usize) -> Vec<StyledVisLine> {
    lines
        .into_iter()
        .enumerate()
        .map(|(row, line)| {
            let tag = spectrum_row_tag(row, total_rows);
            StyledVisLine {
                tags: vec![tag; line.chars().count()],
                text: line,
            }
        })
        .collect()
}

fn spectrum_row_tag(row: usize, total: usize) -> u8 {
    if total == 0 {
        return 0;
    }
    if row < total / 3 {
        2
    } else if row < (total * 2) / 3 {
        1
    } else {
        0
    }
}

fn tag_color(tag: u8) -> Color {
    match tag {
        2 => Color::LightRed,
        1 => Color::Yellow,
        _ => Color::LightGreen,
    }
}

fn styled_line_to_spans(line: &StyledVisLine) -> Line<'static> {
    let mut spans = Vec::new();
    let mut run_tag: Option<u8> = None;
    let mut run = String::new();
    for (idx, ch) in line.text.chars().enumerate() {
        let tag = *line.tags.get(idx).unwrap_or(&0);
        if run_tag.is_some() && run_tag != Some(tag) {
            spans.push(Span::styled(
                std::mem::take(&mut run),
                Style::default().fg(tag_color(run_tag.unwrap_or(0))),
            ));
        }
        run_tag = Some(tag);
        run.push(ch);
    }
    if !run.is_empty() {
        spans.push(Span::styled(
            run,
            Style::default().fg(tag_color(run_tag.unwrap_or(0))),
        ));
    }
    Line::from(spans)
}

fn vis_band_width(band: usize, total_width: usize, bands: usize) -> usize {
    if bands == 0 {
        return total_width;
    }
    let gap = 1usize;
    let total_gaps = (bands.saturating_sub(1)).saturating_mul(gap);
    let usable = total_width.saturating_sub(total_gaps);
    let base = usable / bands;
    let extra = usable % bands;
    if band < extra { base + 1 } else { base }
}

fn render_bars(bands: [f32; crate::music::NUM_BANDS], width: usize, height: usize) -> Vec<String> {
    let mut lines = vec![String::with_capacity(width); height];
    for row in 0..height {
        let row_bottom = (height.saturating_sub(1).saturating_sub(row)) as f32 / height as f32;
        let row_top = (height.saturating_sub(row)) as f32 / height as f32;
        for b in 0..bands.len() {
            let band_w = vis_band_width(b, width, bands.len());
            let level = bands[b].clamp(0.0, 1.0);
            let ch = frac_block(level, row_bottom, row_top);
            for _ in 0..band_w {
                lines[row].push(ch);
            }
            if b < bands.len() - 1 {
                lines[row].push(' ');
            }
        }
    }
    lines
}

fn render_bricks(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
) -> Vec<String> {
    let mut lines = vec![String::with_capacity(width); height];
    for row in 0..height {
        let row_threshold = (height.saturating_sub(1).saturating_sub(row)) as f32 / height as f32;
        for i in 0..bands.len() {
            let bw = vis_band_width(i, width, bands.len());
            let ch = if bands[i] > row_threshold { '▄' } else { ' ' };
            for _ in 0..bw {
                lines[row].push(ch);
            }
            if i < bands.len() - 1 {
                lines[row].push(' ');
            }
        }
    }
    lines
}

fn render_columns(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
) -> Vec<String> {
    let mut lines = vec![String::with_capacity(width); height];
    let mut cols = Vec::new();
    for b in 0..bands.len() {
        let w = vis_band_width(b, width, bands.len());
        let next = if b + 1 < bands.len() {
            bands[b + 1]
        } else {
            bands[b]
        };
        for c in 0..w {
            let t = c as f32 / (w.max(1) as f32);
            cols.push((bands[b] * (1.0 - t) + next * t).clamp(0.0, 1.0));
        }
        if b < bands.len() - 1 {
            cols.push(0.0);
        }
    }
    for row in 0..height {
        let row_bottom = (height.saturating_sub(1).saturating_sub(row)) as f32 / height as f32;
        let row_top = (height.saturating_sub(row)) as f32 / height as f32;
        for (x, level) in cols.iter().enumerate() {
            let _ = x;
            let ch = frac_block(*level, row_bottom, row_top);
            lines[row].push(ch);
        }
        truncate_to_char_count(&mut lines[row], width);
    }
    lines
}

const BRAILLE_BITS: [[u32; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];
const BAR_BLOCKS: &[char] = &[' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const MATRIX_CHARS: &[char] = &[
    'ｦ', 'ｧ', 'ｨ', 'ｩ', 'ｪ', 'ｫ', 'ｬ', 'ｭ', 'ｮ', 'ｯ', 'ｰ', 'ｱ', 'ｲ', 'ｳ', 'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｸ',
    'ｹ', 'ｺ', 'ｻ', 'ｼ', 'ｽ', 'ｾ', 'ｿ', 'ﾀ', 'ﾁ', 'ﾂ', 'ﾃ', 'ﾄ', '0', '1', '2', '3', '4', '5', '6',
    '7', '8', '9',
];

fn render_braille_wave(samples: &[f32], width: usize, height: usize) -> Vec<String> {
    let dot_rows = height * 4;
    let dot_cols = width * 2;
    let mut ypos = vec![dot_rows / 2; dot_cols];
    if !samples.is_empty() {
        for (x, y) in ypos.iter_mut().enumerate() {
            let idx = x * samples.len() / dot_cols.max(1);
            let sample = samples[idx.min(samples.len() - 1)].clamp(-1.0, 1.0);
            *y = (((1.0 - sample) * (dot_rows.saturating_sub(1) as f32) / 2.0).round() as usize)
                .min(dot_rows.saturating_sub(1));
        }
    }
    let mut lines = vec![String::with_capacity(width); height];
    for row in 0..height {
        for ch in 0..width {
            let mut braille = 0x2800u32;
            for dc in 0..2 {
                let x = ch * 2 + dc;
                if x >= dot_cols {
                    continue;
                }
                let y = ypos[x];
                let prev_y = if x > 0 { ypos[x - 1] } else { y };
                let y_min = y.min(prev_y);
                let y_max = y.max(prev_y);
                for dr in 0..4 {
                    let dot_y = row * 4 + dr;
                    if dot_y >= y_min && dot_y <= y_max {
                        braille |= BRAILLE_BITS[dr][dc];
                    }
                }
            }
            lines[row].push(char::from_u32(braille).unwrap_or(' '));
        }
    }
    lines
}

fn render_braille_scatter(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<String> {
    let dot_rows = height * 4;
    let mut lines = vec![String::with_capacity(width); height];
    for row in 0..height {
        for b in 0..bands.len() {
            let band_w = vis_band_width(b, width, bands.len());
            for c in 0..band_w {
                let mut braille = 0x2800u32;
                for dr in 0..4 {
                    for dc in 0..2 {
                        let dot_row = row * 4 + dr;
                        let dot_col = c * 2 + dc;
                        let h = scatter_hash(b, dot_row, dot_col, frame);
                        let height_factor = 0.5 + 0.5 * (dot_row as f32 / dot_rows.max(1) as f32);
                        let threshold = bands[b] * bands[b] * height_factor;
                        if h < threshold {
                            braille |= BRAILLE_BITS[dr][dc];
                        }
                    }
                }
                lines[row].push(char::from_u32(braille).unwrap_or(' '));
            }
            if b < bands.len() - 1 {
                lines[row].push(' ');
            }
        }
        truncate_to_char_count(&mut lines[row], width);
    }
    lines
}

fn render_braille_flame(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<String> {
    let dot_rows = height * 4;
    let mut lines = vec![String::with_capacity(width); height];
    for row in 0..height {
        for b in 0..bands.len() {
            let chars_per_band = vis_band_width(b, width, bands.len());
            let band_dot_cols = chars_per_band * 2;
            for c in 0..chars_per_band {
                let mut braille = 0x2800u32;
                for dr in 0..4 {
                    for dc in 0..2 {
                        let dot_row = row * 4 + dr;
                        let dot_col = c * 2 + dc;
                        let flame_y = (dot_rows.saturating_sub(1).saturating_sub(dot_row)) as f32
                            / dot_rows.max(1) as f32;
                        if flame_y > bands[b] {
                            continue;
                        }
                        let t = frame as f32 * 0.3;
                        let wobble = (t + flame_y * 6.0 + b as f32 * 2.1).sin() * 1.5;
                        let center_col = band_dot_cols as f32 / 2.0;
                        let tip_narrow = 1.0 - flame_y / bands[b].max(0.01);
                        let flame_width = (0.3 + 0.7 * tip_narrow) * center_col;
                        let dist = ((dot_col as f32) - center_col + 0.5 - wobble).abs();
                        if dist < flame_width {
                            let edge = dist / flame_width.max(0.001);
                            if edge < 0.7 || scatter_hash(b, dot_row, dot_col, frame) < 0.6 {
                                braille |= BRAILLE_BITS[dr][dc];
                            }
                        }
                    }
                }
                lines[row].push(char::from_u32(braille).unwrap_or(' '));
            }
            if b < bands.len() - 1 {
                lines[row].push(' ');
            }
        }
        truncate_to_char_count(&mut lines[row], width);
    }
    lines
}

fn render_braille_pulse(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<StyledVisLine> {
    let dot_rows = height * 4;
    let dot_cols = width * 2;
    let center_x = dot_cols as f32 / 2.0;
    let center_y = dot_rows as f32 / 2.0;
    let x_scale = center_y / center_x.max(1.0);
    let max_r = center_y - 1.0;
    let avg_energy = bands.iter().sum::<f32>() / bands.len().max(1) as f32;
    let shock_phase = ((frame as f32 * 0.10) % 1.0).abs();
    let shock_r = max_r * (0.3 + 0.7 * shock_phase);
    let shock_strength = avg_energy * avg_energy * (1.0 - shock_phase * shock_phase);
    let breath = (frame as f32 * 0.05).sin() * 0.02;
    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let mut line = String::with_capacity(width);
        let mut tags = Vec::with_capacity(width);
        for c in 0..width {
            let mut braille = 0x2800u32;
            let mut max_norm = 0.0f32;
            for dr in 0..4 {
                for dc in 0..2 {
                    let dot_x = (c * 2 + dc) as f32;
                    let dot_y = (row * 4 + dr) as f32;
                    let dx = (dot_x - center_x) * x_scale;
                    let dy = dot_y - center_y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let mut angle = dy.atan2(dx);
                    if angle < 0.0 {
                        angle += 2.0 * std::f32::consts::PI;
                    }
                    let mut rot_angle = angle + frame as f32 * (0.015 + avg_energy * 0.04);
                    rot_angle %= 2.0 * std::f32::consts::PI;
                    let band_pos = rot_angle / (2.0 * std::f32::consts::PI) * bands.len() as f32;
                    let band_idx = (band_pos.floor() as usize) % bands.len();
                    let next_band = (band_idx + 1) % bands.len();
                    let frac = band_pos - band_pos.floor();
                    let t = (1.0 - (frac * std::f32::consts::PI).cos()) / 2.0;
                    let energy = bands[band_idx] * (1.0 - t) + bands[next_band] * t;
                    let blended = energy * 0.6 + avg_energy * 0.4;
                    let punch = blended * blended;
                    let r = max_r * (0.08 + breath + 0.92 * punch);
                    let mut on = r > 0.5 && dist <= r;
                    if !on && r > 0.5 && dist < r + 1.5 {
                        let edge_fade = 1.0 - (dist - r) / 1.5;
                        on = scatter_hash(band_idx, row * 4 + dr, c * 2 + dc, frame)
                            < edge_fade * 0.7;
                    }
                    if !on && shock_strength > 0.05 {
                        let shock_dist = (dist - shock_r).abs();
                        let shock_thick = 0.6 + shock_strength * 1.5;
                        if shock_dist < shock_thick {
                            let fade = 1.0 - shock_dist / shock_thick;
                            on = fade > 0.4;
                        }
                    }
                    if on {
                        braille |= BRAILLE_BITS[dr][dc];
                        if r > 0.5 {
                            max_norm = max_norm.max((dist / r).clamp(0.0, 1.0));
                        }
                    }
                }
            }
            line.push(char::from_u32(braille).unwrap_or(' '));
            tags.push(spec_tag(max_norm));
        }
        lines.push(StyledVisLine { text: line, tags });
    }
    lines
}

fn render_braille_retro(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<StyledVisLine> {
    let dot_rows = height * 4;
    let dot_cols = width * 2;
    let mut grid = vec![0u8; dot_rows * dot_cols];
    let mut tags = vec![0u8; dot_rows * dot_cols];
    let horizon = (dot_rows * 2 / 5).max(2);
    let floor_rows = dot_rows.saturating_sub(horizon);
    let center_x = (dot_cols.saturating_sub(1)) as f32 / 2.0;
    let sun_r = horizon as f32 * 0.85;

    for dy in 0..horizon {
        let row_dist = (horizon - dy) as f32;
        if row_dist > sun_r {
            continue;
        }
        let half_w = (sun_r * sun_r - row_dist * row_dist).sqrt();
        if row_dist < sun_r * 0.5 {
            let sw = (sun_r * 0.15).round().max(1.0) as usize;
            if (row_dist as usize / sw) % 2 == 1 {
                continue;
            }
        }
        let left = (center_x - half_w).round().max(0.0) as usize;
        let right = (center_x + half_w)
            .round()
            .min((dot_cols.saturating_sub(1)) as f32) as usize;
        for dx in left..=right {
            let idx = dy * dot_cols + dx;
            grid[idx] = 1;
            tags[idx] = 1;
        }
    }

    if horizon < dot_rows {
        for dx in 0..dot_cols {
            let idx = horizon * dot_cols + dx;
            grid[idx] = 1;
            tags[idx] = 0;
        }
    }

    for i in 0..=18 {
        let bottom_x = i as f32 * (dot_cols.saturating_sub(1)) as f32 / 18.0;
        for dy in horizon.saturating_add(1)..dot_rows {
            let t = (dy.saturating_sub(horizon)) as f32 / floor_rows.max(1) as f32;
            let screen_x = center_x + (bottom_x - center_x) * t;
            let ix = screen_x.round() as i32;
            if ix >= 0 && (ix as usize) < dot_cols {
                let idx = dy * dot_cols + ix as usize;
                grid[idx] = 1;
                if tags[idx] < 2 {
                    tags[idx] = 0;
                }
            }
        }
    }

    let scroll = ((frame as f32) * 0.08) % 1.0;
    for i in 0..10 {
        let mut z = (i as f32 + scroll) / 10.0;
        if z > 1.0 {
            z -= 1.0;
        }
        let dy = horizon.saturating_add(1)
            + (z * z * floor_rows.saturating_sub(2) as f32).round() as usize;
        if dy > horizon && dy < dot_rows {
            for dx in 0..dot_cols {
                let idx = dy * dot_cols + dx;
                grid[idx] = 1;
                if tags[idx] < 2 {
                    tags[idx] = 0;
                }
            }
        }
    }

    let max_wave = horizon as f32 * 0.85;
    let mut wave_y = vec![horizon; dot_cols];
    for (dx, wy_cell) in wave_y.iter_mut().enumerate().take(dot_cols) {
        let band_f = dx as f32 / dot_cols.saturating_sub(1).max(1) as f32
            * bands.len().saturating_sub(1) as f32;
        let bi = band_f.floor() as usize;
        let frac = band_f - bi as f32;
        let t = (1.0 - (frac * std::f32::consts::PI).cos()) / 2.0;
        let level = if bi + 1 < bands.len() {
            bands[bi] * (1.0 - t) + bands[bi + 1] * t
        } else {
            bands[bi]
        }
        .max(0.03);
        let wy = horizon as i32 - (level * max_wave).round() as i32;
        *wy_cell = wy.clamp(0, dot_rows.saturating_sub(1) as i32) as usize;
    }
    for dx in 0..dot_cols {
        let y = wave_y[dx];
        let idx = y * dot_cols + dx;
        grid[idx] = 1;
        tags[idx] = 2;
        if dx > 0 {
            let lo = y.min(wave_y[dx - 1]);
            let hi = y.max(wave_y[dx - 1]);
            for fy in lo..=hi {
                let fi = fy * dot_cols + dx;
                grid[fi] = 1;
                tags[fi] = 2;
            }
        }
    }

    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let mut line = String::with_capacity(width);
        let mut line_tags = Vec::with_capacity(width);
        for ch in 0..width {
            let mut braille = 0x2800u32;
            let mut cell_tag = 0u8;
            for dr in 0..4 {
                for dc in 0..2 {
                    let dy = row * 4 + dr;
                    let dx = ch * 2 + dc;
                    if dy >= dot_rows || dx >= dot_cols {
                        continue;
                    }
                    let idx = dy * dot_cols + dx;
                    if grid[idx] != 0 {
                        braille |= BRAILLE_BITS[dr][dc];
                        cell_tag = cell_tag.max(tags[idx]);
                    }
                }
            }
            let chr = char::from_u32(braille).unwrap_or(' ');
            line.push(chr);
            line_tags.push(cell_tag);
        }
        lines.push(StyledVisLine {
            text: line,
            tags: line_tags,
        });
    }
    lines
}

fn render_matrix_styled(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<StyledVisLine> {
    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let mut line = String::with_capacity(width);
        let mut tags = Vec::with_capacity(width);
        let mut col = 0usize;
        for (b, _) in bands.iter().enumerate() {
            let w = vis_band_width(b, width, bands.len());
            for _ in 0..w {
                let energy = bands[b];
                let seed = col as u64 * 7919 + 104_729;
                if scatter_hash(b, 0, col, frame / 20) > energy * 1.5 + 0.1 {
                    line.push(' ');
                    tags.push(0);
                    col += 1;
                    continue;
                }
                let speed = 2 + (seed % 3) as usize;
                let trail_len = 3 + ((seed / 7) % 3) as usize;
                let cycle_len = height + trail_len + 4;
                let offset = ((seed / 13) % cycle_len as u64) as usize;
                let pos = ((frame as usize) / speed + offset) % cycle_len;
                let dist = pos as i32 - row as i32;
                if dist < 0 || dist > trail_len as i32 {
                    line.push(' ');
                    tags.push(0);
                } else {
                    let char_seed = seed ^ (row as u64 * 31 + (frame / 4) * 17);
                    line.push(MATRIX_CHARS[char_seed as usize % MATRIX_CHARS.len()]);
                    let tag = if dist == 0 {
                        2
                    } else if dist <= 2 {
                        1
                    } else {
                        0
                    };
                    tags.push(tag);
                }
                col += 1;
            }
            if b < bands.len() - 1 {
                line.push(' ');
                tags.push(0);
                col += 1;
            }
        }
        truncate_to_char_count(&mut line, width);
        tags.truncate(line.chars().count());
        lines.push(StyledVisLine { text: line, tags });
    }
    lines
}

fn render_binary_styled(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<StyledVisLine> {
    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let mut line = String::with_capacity(width);
        let mut tags = Vec::with_capacity(width);
        let mut col = 0usize;
        for (b, _) in bands.iter().enumerate() {
            let w = vis_band_width(b, width, bands.len());
            for _ in 0..w {
                let energy = bands[b];
                let speed = (4_i32 - (energy * 3.0) as i32).max(1) as usize;
                let scroll = frame as usize / speed;
                let h = scatter_hash(b, row + scroll, col, 0);
                let one_prob = energy * 0.6 + 0.15;
                let bit_one = h < one_prob;
                line.push(if bit_one { '1' } else { '0' });
                let tag = if bit_one && energy > 0.4 {
                    2
                } else if bit_one || energy > 0.3 {
                    1
                } else {
                    0
                };
                tags.push(tag);
                col += 1;
            }
            if b < bands.len() - 1 {
                line.push(' ');
                tags.push(0);
                col += 1;
            }
        }
        truncate_to_char_count(&mut line, width);
        tags.truncate(line.chars().count());
        lines.push(StyledVisLine { text: line, tags });
    }
    lines
}

fn render_braille_snow(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<String> {
    let dot_rows = height * 4;
    let dot_cols = width * 2;
    let avg_energy = bands.iter().sum::<f32>() / bands.len().max(1) as f32;
    let wind_strength = (frame as f32 * 0.03).sin() * (0.5 + avg_energy * 2.0);
    let mut lines = vec![String::with_capacity(width); height];
    for (row, line) in lines.iter_mut().enumerate().take(height) {
        for ch in 0..width {
            let mut braille = 0x2800u32;
            for dr in 0..4 {
                for dc in 0..2 {
                    let dot_row = row * 4 + dr;
                    let dot_col = ch * 2 + dc;
                    let mut band_idx = dot_col * bands.len() / dot_cols.max(1);
                    if band_idx >= bands.len() {
                        band_idx = bands.len() - 1;
                    }
                    let energy = bands[band_idx];
                    let col_speed = 1 + ((dot_col as u64 * 7919) % 4) as usize;
                    let adjusted_row = dot_row as i32 - (frame as i32 * col_speed as i32 / 3);
                    let wind_drift =
                        (wind_strength * (dot_row as f32) / dot_rows.max(1) as f32) as i32;
                    let adjusted_col = dot_col as i32 - wind_drift;
                    let h = scatter_hash(
                        band_idx,
                        adjusted_row.max(0) as usize,
                        adjusted_col.max(0) as usize,
                        0,
                    );
                    let threshold = 0.015 + energy * 0.05;
                    if h < threshold {
                        braille |= BRAILLE_BITS[dr][dc];
                    }
                }
            }
            line.push(char::from_u32(braille).unwrap_or(' '));
        }
        truncate_to_char_count(line, width);
    }
    lines
}

fn frac_block(level: f32, row_bottom: f32, row_top: f32) -> char {
    if level >= row_top {
        return '█';
    }
    if level > row_bottom {
        let frac = (level - row_bottom) / (row_top - row_bottom);
        let idx = (frac * (BAR_BLOCKS.len().saturating_sub(1)) as f32) as usize;
        return BAR_BLOCKS[idx.min(BAR_BLOCKS.len().saturating_sub(1))];
    }
    ' '
}

fn spec_tag(norm: f32) -> u8 {
    if norm >= 0.6 {
        2
    } else if norm >= 0.3 {
        1
    } else {
        0
    }
}

fn scatter_hash(band: usize, row: usize, col: usize, frame: u64) -> f32 {
    let f = (frame + (row * 3 + col) as u64) / 3;
    let mut h = band as u64 * 7919 + row as u64 * 6271 + col as u64 * 3037 + f * 104_729;
    h ^= h >> 16;
    h = h.wrapping_mul(0x45d9f3b37197344b);
    h ^= h >> 16;
    (h % 10_000) as f32 / 10_000.0
}

fn put(buf: &mut Buffer, x: i16, y: i16, text: &str, color: ratatui::style::Color) {
    if x < 0 || y < 0 {
        return;
    }
    let x = x as u16;
    let y = y as u16;
    if x >= buf.area().right() || y >= buf.area().bottom() {
        return;
    }
    buf[(x, y)]
        .set_symbol(text)
        .set_style(Style::default().fg(color));
}

fn put_centered(buf: &mut Buffer, area: Rect, y: i16, text: &str, color: ratatui::style::Color) {
    let width = UnicodeWidthStr::width(text) as i16;
    let x = area.x as i16 + area.width as i16 / 2 - width / 2;
    put_text(buf, x, y, text, color);
}

fn put_centered_gradient(
    buf: &mut Buffer,
    area: Rect,
    y: i16,
    text: &str,
    from: ratatui::style::Color,
    mid: ratatui::style::Color,
    to: ratatui::style::Color,
) {
    let width = UnicodeWidthStr::width(text) as i16;
    let x = area.x as i16 + area.width as i16 / 2 - width / 2;
    put_text_gradient(buf, x, y, text, from, mid, to);
}

fn put_text(buf: &mut Buffer, x: i16, y: i16, text: &str, color: ratatui::style::Color) {
    for (offset, ch) in text.chars().enumerate() {
        put(buf, x + offset as i16, y, &ch.to_string(), color);
    }
}

fn put_text_gradient(
    buf: &mut Buffer,
    x: i16,
    y: i16,
    text: &str,
    from: ratatui::style::Color,
    mid: ratatui::style::Color,
    to: ratatui::style::Color,
) {
    let chars: Vec<char> = text.chars().collect();
    let last = chars.len().saturating_sub(1).max(1) as f32;
    for (idx, ch) in chars.into_iter().enumerate() {
        if ch == ' ' {
            continue;
        }
        let t = idx as f32 / last;
        let color = gradient3(from, mid, to, t);
        put(buf, x + idx as i16, y, &ch.to_string(), color);
    }
}

fn solidify_figlet_line(line: &str) -> String {
    line.to_string()
}

fn gradient3(
    from: ratatui::style::Color,
    mid: ratatui::style::Color,
    to: ratatui::style::Color,
    t: f32,
) -> ratatui::style::Color {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.5 {
        lerp_color(from, mid, t * 2.0)
    } else {
        lerp_color(mid, to, (t - 0.5) * 2.0)
    }
}

fn lerp_color(
    from: ratatui::style::Color,
    to: ratatui::style::Color,
    t: f32,
) -> ratatui::style::Color {
    match (from, to) {
        (ratatui::style::Color::Rgb(fr, fg, fb), ratatui::style::Color::Rgb(tr, tg, tb)) => {
            let t = t.clamp(0.0, 1.0);
            ratatui::style::Color::Rgb(
                (fr as f32 + (tr as f32 - fr as f32) * t).round() as u8,
                (fg as f32 + (tg as f32 - fg as f32) * t).round() as u8,
                (fb as f32 + (tb as f32 - fb as f32) * t).round() as u8,
            )
        }
        _ => {
            if t < 0.5 {
                from
            } else {
                to
            }
        }
    }
}

fn figlet_lines(text: &str, max_width: usize) -> Vec<String> {
    static FONT: OnceLock<Option<FIGfont>> = OnceLock::new();
    let font = FONT.get_or_init(|| {
        FIGfont::from_content(include_str!("../assets/fonts/slant.flf"))
            .ok()
            .or_else(|| FIGfont::standard().ok())
    });
    let Some(font) = font else {
        return vec![text.to_string()];
    };
    let Some(fig) = font.convert(text) else {
        return vec![text.to_string()];
    };
    fig.to_string()
        .lines()
        .map(str::to_string)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            if UnicodeWidthStr::width(line.as_str()) > max_width {
                line.chars().take(max_width).collect()
            } else {
                line
            }
        })
        .collect()
}

struct BrailleCanvas {
    area: Rect,
    width_cells: usize,
    height_cells: usize,
    width_sub: usize,
    height_sub: usize,
    bits: Vec<u8>,
    colors: Vec<ratatui::style::Color>,
}

impl BrailleCanvas {
    fn new(area: Rect) -> Self {
        let width_cells = area.width as usize;
        let height_cells = area.height as usize;
        let size = width_cells.saturating_mul(height_cells);
        Self {
            area,
            width_cells,
            height_cells,
            width_sub: width_cells.saturating_mul(2),
            height_sub: height_cells.saturating_mul(4),
            bits: vec![0; size],
            colors: vec![ratatui::style::Color::Reset; size],
        }
    }

    fn plot(&mut self, x_sub: i32, y_sub: i32, color: ratatui::style::Color) {
        if x_sub < 0
            || y_sub < 0
            || x_sub >= self.width_sub as i32
            || y_sub >= self.height_sub as i32
        {
            return;
        }

        let cell_x = (x_sub / 2) as usize;
        let cell_y = (y_sub / 4) as usize;
        let sub_x = (x_sub % 2) as usize;
        let sub_y = (y_sub % 4) as usize;
        let idx = cell_y * self.width_cells + cell_x;
        self.bits[idx] |= braille_mask(sub_x, sub_y);
        self.colors[idx] = color;
    }

    fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: ratatui::style::Color) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let steps = dx.abs().max(dy.abs()).max(1.0) as usize;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = x0 + dx * t;
            let y = y0 + dy * t;
            self.plot(x.round() as i32, y.round() as i32, color);
        }
    }

    fn draw_ellipse(
        &mut self,
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        color: ratatui::style::Color,
        step_degrees: f32,
    ) {
        let mut deg: f32 = 0.0;
        while deg < 360.0 {
            let theta = deg.to_radians();
            let x = cx + theta.cos() * rx;
            let y = cy + theta.sin() * ry;
            self.plot(x.round() as i32, y.round() as i32, color);
            deg += step_degrees;
        }
    }

    fn render(self, buf: &mut Buffer) {
        for y in 0..self.height_cells {
            for x in 0..self.width_cells {
                let idx = y * self.width_cells + x;
                let bits = self.bits[idx];
                if bits == 0 {
                    continue;
                }
                let ch = char::from_u32(0x2800 + bits as u32).unwrap_or(' ');
                let ux = self.area.x + x as u16;
                let uy = self.area.y + y as u16;
                if ux < buf.area().right() && uy < buf.area().bottom() {
                    buf[(ux, uy)]
                        .set_symbol(&ch.to_string())
                        .set_style(Style::default().fg(self.colors[idx]));
                }
            }
        }
    }
}

fn braille_mask(sub_x: usize, sub_y: usize) -> u8 {
    match (sub_x, sub_y) {
        (0, 0) => 0x01,
        (0, 1) => 0x02,
        (0, 2) => 0x04,
        (0, 3) => 0x40,
        (1, 0) => 0x08,
        (1, 1) => 0x10,
        (1, 2) => 0x20,
        (1, 3) => 0x80,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;

    use super::*;
    use crate::{
        app::ModeKind,
        modes::clock::ClockSnapshot,
        music::{MusicSnapshot, PlaybackState, VisualizerMode},
        theme::Theme,
    };

    #[test]
    fn dashboard_renders_key_text() {
        let area = Rect::new(0, 0, 120, 36);
        let mut buffer = Buffer::empty(area);
        let view = DashboardView {
            mode: ModeKind::Pomodoro,
            clock: &ClockSnapshot {
                time_text: "12:34:56".into(),
                date_text: "Sun, Mar 08".into(),
                hour_angle: 0.0,
                minute_angle: 0.5,
                second_angle: 0.75,
            },
            pomodoro: PomodoroSnapshot {
                phase: crate::modes::pomodoro::PhaseKind::Work,
                remaining: std::time::Duration::from_secs(12 * 60),
                progress: 0.5,
                running: true,
                completed: false,
                cycle: 2,
            },
            music: &MusicSnapshot {
                state: PlaybackState::Idle,
                ..MusicSnapshot::default()
            },
            music_full_visualizer: false,
            music_queue_overlay: false,
            pose: SpritePose::default(),
            theme: Theme::default(),
            system: SystemStats {
                cpu_usage: 13.5,
                memory_used_mib: 1024,
                memory_total_mib: 8192,
            },
        };
        view.render(area, &mut buffer);
        let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("Braille Dial"));
        assert!(text.contains("12:34:56"));
        assert!(text.contains("CPU"));
    }

    #[test]
    fn truncate_handles_multibyte_chars() {
        let mut s = "ｱｲｳｴｵ".to_string();
        truncate_to_char_count(&mut s, 3);
        assert_eq!(s, "ｱｲｳ");
    }

    #[test]
    fn music_visualizer_modes_render_without_panic() {
        let area = Rect::new(0, 0, 120, 36);
        for mode in [
            VisualizerMode::Wave,
            VisualizerMode::Scatter,
            VisualizerMode::Flame,
            VisualizerMode::Pulse,
            VisualizerMode::Retro,
            VisualizerMode::Snow,
            VisualizerMode::Matrix,
            VisualizerMode::Binary,
        ] {
            let mut buffer = Buffer::empty(area);
            let snapshot = MusicSnapshot {
                state: PlaybackState::Playing,
                visualizer_mode: mode,
                spectrum_bands: [0.4; crate::music::NUM_BANDS],
                wave_samples: (0..2048)
                    .map(|i| ((i as f32) * 0.01).sin())
                    .collect::<Vec<_>>(),
                ..MusicSnapshot::default()
            };
            let view = DashboardView {
                mode: ModeKind::Music,
                clock: &ClockSnapshot {
                    time_text: "12:34:56".into(),
                    date_text: "Sun, Mar 08".into(),
                    hour_angle: 0.0,
                    minute_angle: 0.5,
                    second_angle: 0.75,
                },
                pomodoro: PomodoroSnapshot {
                    phase: crate::modes::pomodoro::PhaseKind::Work,
                    remaining: std::time::Duration::from_secs(12 * 60),
                    progress: 0.5,
                    running: true,
                    completed: false,
                    cycle: 2,
                },
                music: &snapshot,
                music_full_visualizer: false,
                music_queue_overlay: false,
                pose: SpritePose::default(),
                theme: Theme::default(),
                system: SystemStats {
                    cpu_usage: 13.5,
                    memory_used_mib: 1024,
                    memory_total_mib: 8192,
                },
            };
            view.render(area, &mut buffer);
        }
    }
}
