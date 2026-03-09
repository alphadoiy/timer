use super::*;

pub(super) fn progress_bar(progress: f32, width: usize) -> String {
    let filled = (progress.clamp(0.0, 1.0) * width as f32).round() as usize;
    let mut out = String::with_capacity(width);
    for idx in 0..width {
        out.push(if idx < filled { '●' } else { '·' });
    }
    out
}

pub(super) fn build_visual_progress_line(
    width: usize,
    progress: Option<f32>,
    playback_secs: f32,
) -> Line<'static> {
    if width == 0 {
        return Line::from(String::new());
    }

    if width == 1 {
        return Line::from(Span::styled("◉", Style::default().fg(Color::Rgb(255, 150, 170))));
    }

    let inner = width.saturating_sub(2).max(1);
    let mut spans = Vec::with_capacity(width);
    let head = progress.map(|p| (p * inner as f32).round() as usize).unwrap_or(0);
    let unknown_head = ((playback_secs * 8.0) as usize) % inner;

    spans.push(Span::styled(
        "╞",
        Style::default().fg(Color::Rgb(92, 214, 188)),
    ));

    for x in 0..inner {
        let phase = playback_secs * 4.0;
        let spatial = (x as f32 / inner.max(1) as f32) * std::f32::consts::TAU * 7.5;
        let pulse = ((phase + spatial).sin() * 0.5 + 0.5).clamp(0.0, 1.0);

        let (ch, color) = if let Some(_) = progress {
            if x == head.min(inner.saturating_sub(1)) {
                ('◉', Color::Rgb(255, 138, 172))
            } else if x < head {
                let fill = progress_wave_char((0.30 + pulse * 0.70).clamp(0.0, 1.0));
                let t = x as f32 / inner.max(1) as f32;
                (
                    fill,
                    lerp_color(
                        Color::Rgb(76, 218, 196),
                        Color::Rgb(255, 184, 112),
                        t,
                    ),
                )
            } else {
                (if x % 2 == 0 { '─' } else { '·' }, Color::Rgb(84, 118, 132))
            }
        } else if x == unknown_head {
            ('◉', Color::Rgb(255, 144, 176))
        } else if x + 1 == unknown_head || x == unknown_head + 1 {
            ('•', Color::Rgb(230, 188, 128))
        } else {
            (
                if x % 2 == 0 { '─' } else { '·' },
                Color::Rgb(90, 174, 176),
            )
        };

        spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
    }

    spans.push(Span::styled(
        "╡",
        Style::default().fg(Color::Rgb(255, 176, 108)),
    ));
    Line::from(spans)
}

pub(super) fn format_duration(duration: std::time::Duration) -> String {
    let total = duration.as_secs();
    let minutes = total / 60;
    let seconds = total % 60;
    format!("{minutes:02}:{seconds:02}")
}

fn progress_wave_char(level: f32) -> char {
    const RAMP: [char; 12] = ['⠁', '⠃', '⠇', '⠏', '⠟', '⠿', '⡿', '⣿', '⣷', '⣾', '⣽', '⣿'];
    let idx = (level.clamp(0.0, 1.0) * (RAMP.len().saturating_sub(1)) as f32).round() as usize;
    RAMP[idx.min(RAMP.len().saturating_sub(1))]
}
