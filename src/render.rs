use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::{
    animation::SpritePose,
    app::{ModeKind, SystemStats},
    modes::{clock::ClockSnapshot, pomodoro::PomodoroSnapshot},
    music::{MusicSnapshot, ui as music_ui},
    theme::Theme,
};

mod canvas;
mod clock;
mod draw;
mod music;
mod pomodoro;
mod progress;
#[cfg(test)]
mod tests;
mod visualizer;
mod weathr;

pub(super) use canvas::BrailleCanvas;
use draw::*;
use progress::*;
use visualizer::*;

pub struct DashboardView<'a> {
    pub mode: ModeKind,
    pub clock: &'a ClockSnapshot,
    pub pomodoro: PomodoroSnapshot,
    pub music: &'a MusicSnapshot,
    pub music_full_visualizer: bool,
    pub music_queue_overlay: bool,
    pub pose: SpritePose,
    pub theme: Theme,
    pub dark_bg: bool,
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
                    self.dark_bg,
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
                " {mode} | CPU {:>5.1}% | MEM {:>4}/{:>4} MiB | {} | [Tab/←/→] switch  [Space] start/pause  [r] reset  [b] bg  [q] quit ",
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

}
