use super::*;
use std::sync::OnceLock;

use figlet_rs::FIGfont;
use unicode_width::UnicodeWidthStr;

pub(super) fn render_figlet_time(area: Rect, buf: &mut Buffer, text: &str, theme: Theme) {
    render_figlet_time_shifted(area, buf, text, theme, 0, 0);
}

pub(super) fn render_figlet_time_shifted(
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

pub(super) fn box_block<'a>(title: &'a str, theme: Theme) -> Block<'a> {
    Block::default()
        .title(Line::from(Span::styled(
            format!(" {} ", title),
            Style::default().fg(theme.accent_soft),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.outline))
}

pub(super) fn centered_rect(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
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

pub(super) fn put(buf: &mut Buffer, x: i16, y: i16, text: &str, color: ratatui::style::Color) {
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

pub(super) fn put_centered(
    buf: &mut Buffer,
    area: Rect,
    y: i16,
    text: &str,
    color: ratatui::style::Color,
) {
    let width = UnicodeWidthStr::width(text) as i16;
    let x = area.x as i16 + area.width as i16 / 2 - width / 2;
    put_text(buf, x, y, text, color);
}

pub(super) fn put_centered_gradient(
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

pub(super) fn put_text(buf: &mut Buffer, x: i16, y: i16, text: &str, color: ratatui::style::Color) {
    for (offset, ch) in text.chars().enumerate() {
        put(buf, x + offset as i16, y, &ch.to_string(), color);
    }
}

pub(super) fn put_text_gradient(
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

pub(super) fn solidify_figlet_line(line: &str) -> String {
    line.to_string()
}

pub(super) fn gradient3(
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

pub(super) fn lerp_color(
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

pub(super) fn figlet_lines(text: &str, max_width: usize) -> Vec<String> {
    static FONT: OnceLock<Option<FIGfont>> = OnceLock::new();
    let font = FONT.get_or_init(|| {
        FIGfont::from_content(include_str!("../../assets/fonts/slant.flf"))
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
