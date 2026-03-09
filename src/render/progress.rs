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
    height: usize,
) -> Line<'static> {
    if width == 0 {
        return Line::from(String::new());
    }

    if width == 1 {
        return Line::from(Span::styled("◉", Style::default().fg(Color::Rgb(255, 150, 170))));
    }

    let inner = width.saturating_sub(2).max(1);
    let head = progress.map(|p| (p * inner as f32).round() as usize).unwrap_or(0);
    let unknown_head = ((playback_secs * 8.0) as usize) % inner;
    let y_gain = (height.max(1) as f32).sqrt().clamp(1.0, 2.2);
    let frame = (playback_secs * 60.0) as u64;
    let mut spans = Vec::with_capacity(width);
    spans.push(Span::styled(
        "╞",
        Style::default().fg(Color::Rgb(92, 214, 188)),
    ));

    for x in 0..inner {
        let t = x as f32 / inner as f32;
        let travel = playback_secs * 4.8;
        let swell = ((t * std::f32::consts::TAU * 1.7) - travel).sin() * 0.5 + 0.5;
        let chop =
            ((t * std::f32::consts::TAU * 9.8) + travel * 1.9 + t * 3.7).sin() * 0.5 + 0.5;
        let counter =
            (((1.0 - t) * std::f32::consts::TAU * 3.2) - travel * 1.3).sin() * 0.5 + 0.5;
        let turbulence = (noise01(x as u64, frame) - 0.5) * 0.34;
        let surface = (swell * 0.46 + chop * 0.32 + counter * 0.22 + turbulence).clamp(0.0, 1.0);

        let (ch, color) = if let Some(_) = progress {
            let head = head.min(inner.saturating_sub(1));
            if x == head {
                ('◉', Color::Rgb(255, 138, 172))
            } else if x < head {
                let dist = (head - x) as f32 / inner as f32;
                let wake = (1.0 - dist * 5.0).clamp(0.0, 1.0);
                let crest_boost = wake * 0.34 * y_gain;
                let body = (surface * (0.82 + y_gain * 0.28)).clamp(0.0, 1.0);
                let level = (body + crest_boost).clamp(0.0, 1.0);
                let next_t = ((x + 1).min(inner.saturating_sub(1))) as f32 / inner as f32;
                let next_swell =
                    ((next_t * std::f32::consts::TAU * 1.7) - travel).sin() * 0.5 + 0.5;
                let next_chop = ((next_t * std::f32::consts::TAU * 9.8)
                    + travel * 1.9
                    + next_t * 3.7)
                    .sin()
                    * 0.5
                    + 0.5;
                let next_counter =
                    (((1.0 - next_t) * std::f32::consts::TAU * 3.2) - travel * 1.3).sin() * 0.5
                        + 0.5;
                let next_turbulence = (noise01((x + 1) as u64, frame) - 0.5) * 0.34;
                let next_surface =
                    (next_swell * 0.46 + next_chop * 0.32 + next_counter * 0.22 + next_turbulence)
                        .clamp(0.0, 1.0);
                let next_level = (next_surface * (0.82 + y_gain * 0.28) + crest_boost).clamp(0.0, 1.0);
                let slope = (next_level - level).clamp(-1.0, 1.0);
                let fill = progress_wave_char(level, slope);
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
                // Keep unplayed part static: no wave shape.
                ('─', Color::Rgb(68, 88, 98))
            }
        } else if x == unknown_head {
            ('◉', Color::Rgb(255, 144, 176))
        } else if x + 1 == unknown_head || x == unknown_head + 1 {
            ('•', Color::Rgb(230, 188, 128))
        } else {
            ('─', Color::Rgb(82, 106, 118))
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

fn progress_wave_char(level: f32, slope: f32) -> char {
    // Draw a contour-like braille wave using sparse dots instead of filled blocks.
    let row = ((1.0 - level.clamp(0.0, 1.0)) * 3.0).round() as i32;
    let row = row.clamp(0, 3) as usize;

    let left = match row {
        0 => 0b0000_0001, // dot 1
        1 => 0b0000_0010, // dot 2
        2 => 0b0000_0100, // dot 3
        _ => 0b0100_0000, // dot 7
    };

    let slope_shift = if slope > 0.08 {
        1
    } else if slope < -0.08 {
        -1
    } else {
        0
    };
    let right_row = (row as i32 + slope_shift).clamp(0, 3) as usize;
    let right = match right_row {
        0 => 0b0000_1000,  // dot 4
        1 => 0b0001_0000,  // dot 5
        2 => 0b0010_0000,  // dot 6
        _ => 0b1000_0000,  // dot 8
    };

    // Add thickness near crests to make wave peaks pop.
    let thickness = if level > 0.78 {
        match row {
            0 => 0b0000_0010,
            1 => 0b0000_0100,
            2 => 0b0100_0000,
            _ => 0,
        }
    } else {
        0
    };

    char::from_u32(0x2800 + (left | right | thickness) as u32).unwrap_or('⠒')
}

fn noise01(x: u64, frame: u64) -> f32 {
    let mut n = x
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(frame.wrapping_mul(0xBF58_476D_1CE4_E5B9));
    n ^= n >> 30;
    n = n.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    n ^= n >> 27;
    n = n.wrapping_mul(0x94D0_49BB_1331_11EB);
    n ^= n >> 31;
    (n as f32) / (u64::MAX as f32)
}

// ---------------------------------------------------------------------------
// Braille seek-wave renderer (fills the full 2-row area)
// ---------------------------------------------------------------------------

pub(super) fn render_seek_wave(
    area: Rect,
    buf: &mut Buffer,
    progress: Option<f32>,
    playback_secs: f32,
    spectrum: &[f32],
    vis_frame: u64,
) {
    let width = area.width as usize;
    let rows = area.height as usize;
    if width < 3 || rows == 0 {
        return;
    }
    let inner_width = width.saturating_sub(2);
    if inner_width == 0 {
        return;
    }

    // Unknown duration — fall back to old single-line rendering
    if progress.is_none() {
        let line = build_visual_progress_line(width, None, playback_secs, rows);
        let y = area.y + area.height.saturating_sub(1) / 2;
        Paragraph::new(line).render(
            Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            },
            buf,
        );
        return;
    }

    let progress_val = progress.unwrap();
    let head = ((progress_val * inner_width as f32).round() as usize)
        .min(inner_width.saturating_sub(1));
    let total_sub_cols = (inner_width * 2).max(1);
    let sub_rows = rows * 4;
    let center_f = sub_rows as f32 / 2.0;

    let mut row_spans: Vec<Vec<Span<'static>>> =
        (0..rows).map(|_| Vec::with_capacity(width)).collect();

    // Braille center-line chars for multi-row alignment:
    // Row 0 bottom dots (⣀ U+28C0) + Row 1 top dots (⠉ U+2809) = thin
    // horizontal line at the exact boundary between the two rows, matching
    // the symmetric wave center.
    let center_line_chars: Vec<char> = (0..rows)
        .map(|r| {
            if rows < 2 {
                return '─';
            }
            let mid = rows / 2; // boundary row index (upper side)
            if r + 1 == mid || (mid == 0 && r == 0) {
                // row just above the center boundary → bottom dots
                // bits: dot7 | dot8 = 0x40 | 0x80 = 0xC0
                '\u{28C0}' // ⣀
            } else if r == mid || (mid == 0 && r == 1) {
                // row just below the center boundary → top dots
                // bits: dot1 | dot4 = 0x01 | 0x08 = 0x09
                '\u{2809}' // ⠉
            } else {
                ' '
            }
        })
        .collect();

    // Left cap
    let cap_l_style = Style::default().fg(Color::Rgb(92, 214, 188));
    if rows < 2 {
        row_spans[0].push(Span::styled("╞", cap_l_style));
    } else {
        for r in 0..rows {
            row_spans[r].push(Span::styled(
                center_line_chars[r].to_string(),
                cap_l_style,
            ));
        }
    }

    for cx in 0..inner_width {
        if cx == head {
            let head_style = Style::default().fg(Color::Rgb(255, 138, 172));
            if rows < 2 {
                row_spans[0].push(Span::styled("◉", head_style));
            } else {
                for r in 0..rows {
                    row_spans[r].push(Span::styled(
                        center_line_chars[r].to_string(),
                        head_style,
                    ));
                }
            }
            continue;
        }

        if cx > head {
            let dim_style = Style::default().fg(Color::Rgb(68, 88, 98));
            if rows < 2 {
                row_spans[0].push(Span::styled("─", dim_style));
            } else {
                for r in 0..rows {
                    row_spans[r].push(Span::styled(
                        center_line_chars[r].to_string(),
                        dim_style,
                    ));
                }
            }
            continue;
        }

        // === Played region: symmetric braille waveform ===
        let dist_to_head = (head as f32 - cx as f32) / inner_width.max(1) as f32;
        let wake = (1.0 - dist_to_head * 5.0).clamp(0.0, 1.0);

        let mut cell_bits = vec![0u8; rows];

        for sub_col in 0..2usize {
            let sx = cx * 2 + sub_col;
            let t = sx as f32 / total_sub_cols as f32;

            let band = interpolate_spectrum(t, spectrum);
            let wave = seek_layered_sine(t, playback_secs, sx, vis_frame);
            let amplitude =
                (wave * (0.15 + band * 0.80 + wake * 0.15)).clamp(0.08, 1.0);

            let top = (center_f - amplitude * center_f).max(0.0) as usize;
            let bottom =
                ((center_f + amplitude * center_f).ceil() as usize).min(sub_rows);

            for sub_row in top..bottom {
                let cell_row = sub_row / 4;
                let dot_row = sub_row % 4;
                if cell_row < rows {
                    cell_bits[cell_row] |= seek_braille_mask(sub_col, dot_row);
                }
            }
        }

        // Gradient teal→amber with brightness glow near playhead
        let grad_t = cx as f32 / inner_width.max(1) as f32;
        let base = lerp_color(
            Color::Rgb(76, 218, 196),
            Color::Rgb(255, 184, 112),
            grad_t,
        );
        let color = if wake > 0.0 {
            brighten_color(base, wake * 0.3)
        } else {
            base
        };
        let style = Style::default().fg(color);

        for r in 0..rows {
            let ch = char::from_u32(0x2800 + cell_bits[r] as u32).unwrap_or(' ');
            row_spans[r].push(Span::styled(ch.to_string(), style));
        }
    }

    // Right cap
    let cap_r_style = Style::default().fg(Color::Rgb(255, 176, 108));
    if rows < 2 {
        row_spans[0].push(Span::styled("╡", cap_r_style));
    } else {
        for r in 0..rows {
            row_spans[r].push(Span::styled(
                center_line_chars[r].to_string(),
                cap_r_style,
            ));
        }
    }

    for (r, spans) in row_spans.into_iter().enumerate() {
        Paragraph::new(Line::from(spans)).render(
            Rect {
                x: area.x,
                y: area.y + r as u16,
                width: area.width,
                height: 1,
            },
            buf,
        );
    }
}

fn interpolate_spectrum(t: f32, spectrum: &[f32]) -> f32 {
    if spectrum.is_empty() {
        return 0.0;
    }
    let max_idx = (spectrum.len() - 1) as f32;
    let pos = t * max_idx;
    let idx = pos.floor() as usize;
    let frac = pos - idx as f32;
    let a = spectrum[idx.min(spectrum.len() - 1)];
    let b = spectrum[(idx + 1).min(spectrum.len() - 1)];
    a + (b - a) * frac
}

fn seek_layered_sine(t: f32, playback_secs: f32, sx: usize, frame: u64) -> f32 {
    let travel = playback_secs * 4.8;
    let tau = std::f32::consts::TAU;
    let wave1 = ((t * tau * 1.7) - travel).sin() * 0.5 + 0.5;
    let wave2 = ((t * tau * 9.8) + travel * 1.9 + t * 3.7).sin() * 0.5 + 0.5;
    let wave3 = (((1.0 - t) * tau * 3.2) - travel * 1.3).sin() * 0.5 + 0.5;
    let turbulence = (noise01(sx as u64, frame) - 0.5) * 0.34;
    (wave1 * 0.46 + wave2 * 0.32 + wave3 * 0.22 + turbulence).clamp(0.0, 1.0)
}

fn seek_braille_mask(sub_x: usize, sub_y: usize) -> u8 {
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

fn brighten_color(color: Color, amount: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => {
            let boost = (amount * 255.0).round() as u16;
            Color::Rgb(
                (r as u16 + boost).min(255) as u8,
                (g as u16 + boost).min(255) as u8,
                (b as u16 + boost).min(255) as u8,
            )
        }
        _ => color,
    }
}
