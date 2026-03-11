use std::{
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use unicode_width::UnicodeWidthStr;

use crate::render::weathr::weather_scene::WeatherScene;

use super::*;

pub(crate) fn current_weather_summary() -> String {
    let state = weather_state();
    let state = state.lock().expect("weather animation mutex poisoned");
    let w = state.current_weather;
    let precip = if !w.precipitation_mm.is_finite() || w.precipitation_mm <= 0.05 {
        "0".to_string()
    } else {
        format!("{:.1}", w.precipitation_mm)
    };
    format!(
        "{}  {:.1}C  {:.1}km/h  {}mm",
        w.condition.label(),
        w.temperature_c,
        w.wind_kmh,
        precip
    )
}

pub(super) fn render_pomodoro_road_panel(
    area: Rect,
    buf: &mut Buffer,
    pomodoro: PomodoroSnapshot,
    _pose: SpritePose,
    theme: Theme,
    dark_bg: bool,
) {
    let remaining_secs = pomodoro.remaining.as_secs();
    let countdown = super::format_duration(pomodoro.remaining);
    let finish_sprint = pomodoro.running && remaining_secs <= 10 && !pomodoro.completed;
    let now = phase_seconds();
    let (jx, jy) = if finish_sprint {
        (
            ((now * 48.0).sin() * 1.4).round() as i16,
            ((now * 37.0).cos() * 0.8).round() as i16,
        )
    } else {
        (0, 0)
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(32), Constraint::Percentage(68)])
        .split(area);
    render_pomodoro_countdown(rows[0], buf, &countdown, jx, jy, theme);

    let road_area = rows[1];
    let road_block = Block::default()
        .title(Line::from(Span::styled(
            " Focus Road ",
            Style::default()
                .fg(theme.accent_soft)
                .add_modifier(Modifier::BOLD),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.outline));
    let inner = road_block.inner(road_area);
    road_block.render(road_area, buf);

    if inner.width < 24 || inner.height < 10 {
        Paragraph::new(Line::from(Span::styled(
            "terminal too small for animation",
            Style::default().fg(theme.subtext),
        )))
        .alignment(Alignment::Center)
        .render(inner, buf);
        return;
    }

    let lanes = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(inner);
    let top = lanes[0];
    let world = lanes[1];
    let bottom = lanes[2];

    super::put_text(buf, top.x as i16, top.y as i16, "25:00", theme.text);
    let right_x = top.right() as i16 - UnicodeWidthStr::width("00:00") as i16;
    super::put_text(buf, right_x, top.y as i16, "00:00", theme.text);
    super::put_text(
        buf,
        (right_x - 6).max(top.x as i16),
        top.y as i16,
        "FIN",
        theme.highlight,
    );

    render_weathr_animation(world, buf, finish_sprint, dark_bg);

    Paragraph::new(Line::from(Span::styled(
        "Focus: pixel weather • [b] toggle bg",
        Style::default().fg(theme.subtext),
    )))
    .alignment(Alignment::Center)
    .render(bottom, buf);
}

fn render_weathr_animation(area: Rect, buf: &mut Buffer, finish_sprint: bool, dark_bg: bool) {
    let state = weather_state();
    let mut state = state.lock().expect("weather animation mutex poisoned");
    state.update(area);
    state.render(area, buf, finish_sprint, dark_bg);
}

fn weather_state() -> &'static Mutex<WeatherScene> {
    static STATE: OnceLock<Mutex<WeatherScene>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(WeatherScene::new(80, 20)))
}

fn phase_seconds() -> f32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f32()
}

fn render_pomodoro_countdown(
    area: Rect,
    buf: &mut Buffer,
    text: &str,
    shift_x: i16,
    shift_y: i16,
    theme: Theme,
) {
    let mut lines = super::figlet_lines(text, area.width as usize);
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
        let solid_line = super::solidify_figlet_line(line);
        super::put_centered_gradient(
            buf,
            shifted,
            y,
            &solid_line,
            theme.highlight,
            theme.accent,
            theme.danger,
        );
    }
}
