use std::sync::OnceLock;

use figlet_rs::FIGfont;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    animation::SpritePose,
    app::{ModeKind, SystemStats},
    modes::{clock::ClockSnapshot, pomodoro::PomodoroSnapshot},
    theme::Theme,
};

mod clock;
mod pomodoro;
mod weathr;

pub struct DashboardView<'a> {
    pub mode: ModeKind,
    pub clock: &'a ClockSnapshot,
    pub pomodoro: PomodoroSnapshot,
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
                "ratatui analog clock + pomodoro",
                Style::default().fg(self.theme.subtext),
            ),
        ]);
        Paragraph::new(title)
            .alignment(Alignment::Center)
            .render(area, buf);
    }

    fn render_body(&self, area: Rect, buf: &mut Buffer) {
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
                },
                Style::default()
                    .fg(self.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            )))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.outline));
        let inner = block.inner(area);
        block.render(area, buf);

        if self.mode == ModeKind::Clock {
            clock::render_clock_visual_panel(inner, buf, self.pose, self.theme, self.clock);
        } else {
            pomodoro::render_pomodoro_road_panel(inner, buf, self.pomodoro, self.pose, self.theme);
        }
    }

    fn render_info_panel(&self, area: Rect, buf: &mut Buffer) {
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
                "Scene preset",
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
                    self.system.cpu_usage, self.system.memory_used_mib, self.system.memory_total_mib
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
        };
        Paragraph::new(Line::from(Span::styled(
            text,
            Style::default().fg(self.theme.subtext),
        )))
        .alignment(Alignment::Center)
        .render(area, buf);
    }

    fn render_status_bar(&self, area: Rect, buf: &mut Buffer) {
        let mode = if self.mode == ModeKind::Clock {
            "CLOCK"
        } else {
            self.pomodoro.phase.label()
        };
        let status = if self.pomodoro.completed {
            "COMPLETED"
        } else if self.pomodoro.running {
            "RUNNING"
        } else {
            "PAUSED"
        };
        let line = format!(
            " {mode} | CPU {:>5.1}% | MEM {:>4}/{:>4} MiB | {} | [Tab/←/→] switch  [Space] start/pause  [r] reset  [q] quit ",
            self.system.cpu_usage,
            self.system.memory_used_mib,
            self.system.memory_total_mib,
            status
        );
        Paragraph::new(Line::from(Span::styled(
            line,
            Style::default()
                .fg(self.theme.text)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .render(area, buf);
    }

    fn weather_label(&self) -> &'static str {
        if self.pomodoro.running && self.pomodoro.remaining.as_secs() <= 10 && !self.pomodoro.completed
        {
            return "Stormy";
        }
        if !self.pomodoro.running {
            return "Foggy";
        }
        match self.pomodoro.cycle % 4 {
            1 => "Sunny",
            2 => "Rainy",
            3 => "Snowy",
            _ => "Sunny",
        }
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
        put_centered(buf, shifted, y, line, theme.figlet_fg);
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

fn put_text(buf: &mut Buffer, x: i16, y: i16, text: &str, color: ratatui::style::Color) {
    for (offset, ch) in text.chars().enumerate() {
        put(buf, x + offset as i16, y, &ch.to_string(), color);
    }
}

fn figlet_lines(text: &str, max_width: usize) -> Vec<String> {
    static FONT: OnceLock<Option<FIGfont>> = OnceLock::new();
    let font = FONT.get_or_init(|| FIGfont::standard().ok());
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
    use crate::{app::ModeKind, modes::clock::ClockSnapshot, theme::Theme};

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
}
