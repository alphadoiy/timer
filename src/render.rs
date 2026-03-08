use std::{
    f32::consts::{FRAC_PI_2, TAU},
    sync::OnceLock,
};

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
                Constraint::Min(16),
                Constraint::Length(2),
            ])
            .split(inner);

        self.render_header(sections[0], buf);
        self.render_body(sections[1], buf);
        self.render_footer(sections[2], buf);
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
                    ModeKind::Pomodoro => " Pomodoro Dial ",
                },
                Style::default()
                    .fg(self.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            )))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.outline));
        let inner = block.inner(area);
        block.render(area, buf);

        let panes = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(78), Constraint::Percentage(22)])
            .split(inner);

        render_braille_dial(
            panes[0],
            buf,
            self.pose,
            self.theme,
            self.mode,
            self.clock,
            self.pomodoro,
        );
        render_figlet_clock(panes[1], buf, self.clock, self.theme);
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
                format!(
                    "CPU {:>5.1}%   MEM {:>4}/{:>4} MiB",
                    self.system.cpu_usage,
                    self.system.memory_used_mib,
                    self.system.memory_total_mib
                ),
                Style::default().fg(self.theme.text),
            )),
            Line::from(Span::styled(
                "Space start/pause   r reset   q quit",
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("DevOps Status", self.theme))
        .alignment(Alignment::Center)
        .render(rows[4], buf);
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
}

fn render_figlet_clock(area: Rect, buf: &mut Buffer, clock: &ClockSnapshot, theme: Theme) {
    let digital = clock.time_text.clone();
    let mut lines = figlet_lines(&digital, area.width as usize);
    if lines.is_empty() {
        lines.push(digital);
    }

    let start_y = area.y + (area.height.saturating_sub(lines.len() as u16)) / 2;
    for (idx, line) in lines.iter().enumerate() {
        let y = start_y + idx as u16;
        if y >= area.bottom() {
            break;
        }
        put_centered(buf, area, y as i16, line, theme.accent);
    }
}

fn render_braille_dial(
    area: Rect,
    buf: &mut Buffer,
    pose: SpritePose,
    theme: Theme,
    mode: ModeKind,
    clock: &ClockSnapshot,
    pomodoro: PomodoroSnapshot,
) {
    let mut canvas = BrailleCanvas::new(area);
    let center_x = canvas.width_sub as f32 / 2.0 + pose.dial_offset_x as f32 * 2.0;
    let center_y = canvas.height_sub as f32 / 2.0 + pose.dial_offset_y as f32 * 4.0;
    let radius =
        (canvas.width_sub.min(canvas.height_sub) as f32 * 0.40 * pose.radius_scale).max(8.0);

    // Slimmed outer rim to keep the contour crisp without feeling too thick.
    canvas.draw_ellipse(
        center_x,
        center_y,
        radius + 0.35,
        radius + 0.35,
        theme.outline,
        1.8,
    );
    canvas.draw_ellipse(
        center_x,
        center_y,
        radius - 0.35,
        radius - 0.35,
        theme.outline,
        1.8,
    );
    canvas.draw_ellipse(
        center_x,
        center_y,
        radius - 2.0,
        radius - 2.0,
        theme.accent_soft,
        4.0,
    );
    draw_dial_ticks(
        &mut canvas,
        center_x,
        center_y,
        radius,
        theme.subtext,
        theme.outline,
    );

    match mode {
        ModeKind::Clock => {
            let second_turn = clock.second_angle + pose.second_sweep + pose.tilt;
            draw_hour_hand(
                &mut canvas,
                center_x,
                center_y,
                radius * 0.50,
                clock.hour_angle + pose.hour_sweep + pose.tilt,
                theme.hour_hand,
            );
            draw_minute_hand(
                &mut canvas,
                center_x,
                center_y,
                radius * 0.78,
                clock.minute_angle + pose.minute_sweep + pose.tilt,
                theme.minute_hand,
            );
            draw_second_hand(
                &mut canvas,
                center_x,
                center_y,
                radius * 0.93,
                second_turn,
                theme.second_hand,
            );
        }
        ModeKind::Pomodoro => {
            let turn = (1.0 - pomodoro.progress) + pose.minute_sweep + pose.tilt;
            draw_hand_with_shadow(
                &mut canvas,
                center_x,
                center_y,
                radius * 0.74,
                turn,
                1.0,
                theme.danger,
                theme.shadow,
            );
            draw_progress_ring(
                &mut canvas,
                center_x,
                center_y,
                radius + 2.0,
                pomodoro.progress,
                pose.ring_pulse,
                if pomodoro.completed {
                    theme.success
                } else {
                    theme.danger
                },
                theme.subtext,
            );
        }
    }

    canvas.render(buf);
    put(
        buf,
        sub_to_cell_x(area, center_x),
        sub_to_cell_y(area, center_y),
        "●",
        theme.accent,
    );
    overlay_dial_labels(buf, area, center_x, center_y, radius, theme);
}

fn draw_dial_ticks(
    canvas: &mut BrailleCanvas,
    cx: f32,
    cy: f32,
    radius: f32,
    mark: ratatui::style::Color,
    cardinal: ratatui::style::Color,
) {
    for i in 0..60 {
        let t = i as f32 / 60.0;
        let theta = t * TAU - FRAC_PI_2;
        let c = theta.cos();
        let s = theta.sin();
        let outer_x = cx + c * radius;
        let outer_y = cy + s * radius;
        let inner_scale = if i % 5 == 0 { 0.80 } else { 0.92 };
        let inner_x = cx + c * radius * inner_scale;
        let inner_y = cy + s * radius * inner_scale;
        if i % 15 == 0 {
            draw_thick_line(canvas, inner_x, inner_y, outer_x, outer_y, 1.6, cardinal);
        } else if i % 5 == 0 {
            draw_thick_line(canvas, inner_x, inner_y, outer_x, outer_y, 1.4, cardinal);
        } else {
            canvas.draw_line(inner_x, inner_y, outer_x, outer_y, mark);
        }
    }
}

fn draw_hand_with_shadow(
    canvas: &mut BrailleCanvas,
    cx: f32,
    cy: f32,
    length: f32,
    turn: f32,
    thickness: f32,
    color: ratatui::style::Color,
    shadow: ratatui::style::Color,
) {
    let (end_x, end_y) = hand_endpoint(cx, cy, length, turn);
    draw_thick_line(
        canvas,
        cx + 1.0,
        cy + 1.0,
        end_x + 1.0,
        end_y + 1.0,
        thickness,
        shadow,
    );
    draw_thick_line(canvas, cx, cy, end_x, end_y, thickness, color);
}

fn draw_hour_hand(
    canvas: &mut BrailleCanvas,
    cx: f32,
    cy: f32,
    length: f32,
    turn: f32,
    core: ratatui::style::Color,
) {
    let theta = turn * TAU - FRAC_PI_2;
    let dx = theta.cos();
    let dy = theta.sin();
    let nx = -dy;
    let ny = dx;
    let shaft_end = length * 0.90;
    draw_thick_line(
        canvas,
        cx,
        cy,
        cx + dx * shaft_end,
        cy + dy * shaft_end,
        3.2,
        core,
    );
    let (tip_x, tip_y) = hand_endpoint(cx, cy, length, turn);
    // Blunt cap to keep hour-hand silhouette distinct from minute-hand needle.
    draw_thick_line(
        canvas,
        tip_x - nx * 1.35,
        tip_y - ny * 1.35,
        tip_x + nx * 1.35,
        tip_y + ny * 1.35,
        0.35,
        core,
    );
}

fn draw_minute_hand(
    canvas: &mut BrailleCanvas,
    cx: f32,
    cy: f32,
    length: f32,
    turn: f32,
    core: ratatui::style::Color,
) {
    let theta = turn * TAU - FRAC_PI_2;
    let dx = theta.cos();
    let dy = theta.sin();
    let nx = -dy;
    let ny = dx;
    let shaft_end = length * 0.94;
    draw_thick_line(
        canvas,
        cx,
        cy,
        cx + dx * shaft_end,
        cy + dy * shaft_end,
        1.05,
        core,
    );
    let (tip_x, tip_y) = hand_endpoint(cx, cy, length, turn);
    let (shaft_x, shaft_y) = hand_endpoint(cx, cy, shaft_end, turn);
    draw_thick_line(
        canvas,
        shaft_x - nx * 0.9,
        shaft_y - ny * 0.9,
        tip_x,
        tip_y,
        0.0,
        core,
    );
    draw_thick_line(
        canvas,
        shaft_x + nx * 0.9,
        shaft_y + ny * 0.9,
        tip_x,
        tip_y,
        0.0,
        core,
    );
}

fn draw_second_hand(
    canvas: &mut BrailleCanvas,
    cx: f32,
    cy: f32,
    length: f32,
    turn: f32,
    color: ratatui::style::Color,
) {
    let theta = turn * TAU - FRAC_PI_2;
    let dx = theta.cos();
    let dy = theta.sin();
    let nx = -dy;
    let ny = dx;
    let steps = length.ceil().max(1.0) as i32;

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let px = cx + dx * length * t;
        let py = cy + dy * length * t;
        let shade = lerp_color(scale_color(color, 0.42), color, t);
        canvas.plot(px.round() as i32, py.round() as i32, shade);

        // Low-luminance side pixels smooth diagonal motion on the braille grid.
        let aa = scale_color(color, 0.18);
        canvas.plot(
            (px + nx * 0.8).round() as i32,
            (py + ny * 0.8).round() as i32,
            aa,
        );
        canvas.plot(
            (px - nx * 0.8).round() as i32,
            (py - ny * 0.8).round() as i32,
            aa,
        );
    }

    let (tip_x, tip_y) = hand_endpoint(cx, cy, length + 1.2, turn);
    canvas.plot(tip_x.round() as i32, tip_y.round() as i32, color);

    let (tail_x, tail_y) = hand_endpoint(cx, cy, -length * 0.16, turn);
    canvas.draw_line(cx, cy, tail_x, tail_y, scale_color(color, 0.55));

    // Counterweight dot for a realistic second-hand balance.
    canvas.plot(tail_x.round() as i32, tail_y.round() as i32, color);
    canvas.plot((tail_x + 1.0).round() as i32, tail_y.round() as i32, color);
    canvas.plot(tail_x.round() as i32, (tail_y + 1.0).round() as i32, color);
    canvas.plot(
        (tail_x + 1.0).round() as i32,
        (tail_y + 1.0).round() as i32,
        color,
    );
}

fn hand_endpoint(cx: f32, cy: f32, length: f32, turn: f32) -> (f32, f32) {
    let theta = turn * TAU - FRAC_PI_2;
    (cx + theta.cos() * length, cy + theta.sin() * length)
}

fn overlay_dial_labels(
    buf: &mut Buffer,
    area: Rect,
    center_x: f32,
    center_y: f32,
    radius: f32,
    theme: Theme,
) {
    let labels = [
        ("12", -FRAC_PI_2),
        ("3", 0.0),
        ("6", FRAC_PI_2),
        ("9", std::f32::consts::PI),
    ];
    for (label, theta) in labels {
        let x = center_x + theta.cos() * radius * 0.68;
        let y = center_y + theta.sin() * radius * 0.68;
        let cell_x = sub_to_cell_x(area, x);
        let cell_y = sub_to_cell_y(area, y);
        if label == "12" || label == "6" {
            put_centered(
                buf,
                Rect::new(area.x, cell_y as u16, area.width, 1),
                cell_y,
                label,
                theme.accent_soft,
            );
        } else {
            put(buf, cell_x, cell_y, label, theme.accent_soft);
        }
    }
}

fn sub_to_cell_x(area: Rect, x_sub: f32) -> i16 {
    area.x as i16 + (x_sub / 2.0).round() as i16
}

fn sub_to_cell_y(area: Rect, y_sub: f32) -> i16 {
    area.y as i16 + (y_sub / 4.0).round() as i16
}

fn scale_color(color: ratatui::style::Color, factor: f32) -> ratatui::style::Color {
    match color {
        ratatui::style::Color::Rgb(r, g, b) => ratatui::style::Color::Rgb(
            ((r as f32) * factor).clamp(0.0, 255.0) as u8,
            ((g as f32) * factor).clamp(0.0, 255.0) as u8,
            ((b as f32) * factor).clamp(0.0, 255.0) as u8,
        ),
        _ => color,
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
        _ => to,
    }
}

fn draw_progress_ring(
    canvas: &mut BrailleCanvas,
    cx: f32,
    cy: f32,
    radius: f32,
    remaining_progress: f32,
    pulse: f32,
    active: ratatui::style::Color,
    inactive: ratatui::style::Color,
) {
    let filled = ((1.0 - remaining_progress).clamp(0.0, 1.0) * 180.0) as usize;
    for idx in 0..180usize {
        let t = idx as f32 / 180.0;
        let theta = t * TAU - FRAC_PI_2;
        let x = cx + theta.cos() * (radius + pulse * 2.5);
        let y = cy + theta.sin() * (radius + pulse * 2.5);
        canvas.plot(
            x.round() as i32,
            y.round() as i32,
            if idx <= filled { active } else { inactive },
        );
    }
}

fn draw_thick_line(
    canvas: &mut BrailleCanvas,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    thickness: f32,
    color: ratatui::style::Color,
) {
    if thickness <= 0.0 {
        canvas.draw_line(x0, y0, x1, y1, color);
        return;
    }

    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let nx = -dy / len;
    let ny = dx / len;
    let steps = thickness.round() as i32;
    for offset in -steps..=steps {
        let shift = offset as f32 * 0.7;
        canvas.draw_line(
            x0 + nx * shift,
            y0 + ny * shift,
            x1 + nx * shift,
            y1 + ny * shift,
            color,
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
        assert!(text.contains("DevOps Status"));
    }
}
