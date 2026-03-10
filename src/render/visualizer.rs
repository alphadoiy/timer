use super::*;

pub(super) fn truncate_to_char_count(s: &mut String, max_chars: usize) {
    if s.chars().count() <= max_chars {
        return;
    }
    if let Some((byte_idx, _)) = s.char_indices().nth(max_chars) {
        s.truncate(byte_idx);
    }
}

#[derive(Debug, Clone)]
pub(super) struct StyledVisLine {
    text: String,
    colors: Vec<Color>,
}

pub(super) fn styled_line_to_spans(line: &StyledVisLine) -> Line<'static> {
    let mut spans = Vec::new();
    let mut run_color: Option<Color> = None;
    let mut run = String::new();
    for (idx, ch) in line.text.chars().enumerate() {
        let color = line.colors.get(idx).copied().unwrap_or(Color::Reset);
        if run_color.is_some() && run_color != Some(color) {
            spans.push(Span::styled(
                std::mem::take(&mut run),
                Style::default().fg(run_color.unwrap_or(Color::Reset)),
            ));
        }
        run_color = Some(color);
        run.push(ch);
    }
    if !run.is_empty() {
        spans.push(Span::styled(
            run,
            Style::default().fg(run_color.unwrap_or(Color::Reset)),
        ));
    }
    Line::from(spans)
}

// Palettes: saturated, mid-to-high brightness to stay visible on dark & light backgrounds.

const SPECTRUM: &[(f32, u8, u8, u8)] = &[
    (0.00, 60, 60, 155),
    (0.14, 50, 110, 220),
    (0.28, 30, 210, 200),
    (0.42, 30, 225, 125),
    (0.57, 160, 235, 70),
    (0.71, 250, 215, 50),
    (0.86, 255, 140, 50),
    (1.00, 255, 70, 150),
];

const FIRE: &[(f32, u8, u8, u8)] = &[
    (0.00, 110, 45, 15), (0.22, 185, 55, 12), (0.48, 255, 105, 18),
    (0.72, 255, 195, 50), (1.00, 255, 225, 100),
];
const OCEAN: &[(f32, u8, u8, u8)] = &[
    (0.00, 45, 65, 135), (0.28, 50, 105, 230), (0.52, 35, 195, 222),
    (0.76, 100, 228, 215), (1.00, 120, 235, 250),
];
const MATRIX_PAL: &[(f32, u8, u8, u8)] = &[
    (0.00, 22, 82, 22), (0.28, 18, 105, 28), (0.52, 22, 185, 48),
    (0.76, 60, 248, 82), (1.00, 105, 250, 125),
];
const CYBER: &[(f32, u8, u8, u8)] = &[
    (0.00, 22, 72, 82), (0.28, 22, 108, 118), (0.52, 28, 200, 188),
    (0.76, 80, 245, 218), (1.00, 125, 250, 232),
];

/// Evaluate a multi-stop gradient, quantised to 25 levels for span batching.
fn pal(stops: &[(f32, u8, u8, u8)], t: f32) -> Color {
    let t = (t.clamp(0.0, 1.0) * 24.0).round() / 24.0;
    for w in stops.windows(2) {
        let (t0, r0, g0, b0) = w[0];
        let (t1, r1, g1, b1) = w[1];
        if t <= t1 {
            let f = ((t - t0) / (t1 - t0).max(0.001)).clamp(0.0, 1.0);
            return Color::Rgb(
                (r0 as f32 + (r1 as f32 - r0 as f32) * f).round() as u8,
                (g0 as f32 + (g1 as f32 - g0 as f32) * f).round() as u8,
                (b0 as f32 + (b1 as f32 - b0 as f32) * f).round() as u8,
            );
        }
    }
    let &(_, r, g, b) = stops.last().unwrap_or(&(1.0, 255, 255, 255));
    Color::Rgb(r, g, b)
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

const BRAILLE_BITS: [[u32; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];
const MATRIX_CHARS: &[char] = &[
    'ｦ', 'ｧ', 'ｨ', 'ｩ', 'ｪ', 'ｫ', 'ｬ', 'ｭ', 'ｮ', 'ｯ', 'ｰ', 'ｱ', 'ｲ', 'ｳ', 'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｸ',
    'ｹ', 'ｺ', 'ｻ', 'ｼ', 'ｽ', 'ｾ', 'ｿ', 'ﾀ', 'ﾁ', 'ﾂ', 'ﾃ', 'ﾄ', '0', '1', '2', '3', '4', '5', '6',
    '7', '8', '9',
];

// --- Bricks: braille sub-pixel bars (4x vertical resolution) ---

pub(super) fn render_bricks_styled(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<StyledVisLine> {
    let total_sub = height * 4;
    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let norm_y = 1.0 - row as f32 / height.saturating_sub(1).max(1) as f32;
        let mut text = String::with_capacity(width);
        let mut colors = Vec::with_capacity(width);
        for i in 0..bands.len() {
            let bw = vis_band_width(i, width, bands.len());
            let fill = (bands[i] * total_sub as f32).round() as usize;
            let bar_top = total_sub.saturating_sub(fill);
            let peak_sub = bar_top.saturating_sub(1);
            for c in 0..bw {
                let mut braille = 0x2800u32;
                let mut any_lit = false;
                for dy in 0..4 {
                    let gy = row * 4 + dy;
                    let is_bar = fill > 0 && gy >= bar_top;
                    let is_peak = bands[i] > 0.05 && fill < total_sub && gy == peak_sub;
                    if is_bar || is_peak {
                        braille |= BRAILLE_BITS[dy][0] | BRAILLE_BITS[dy][1];
                        any_lit = true;
                    }
                }
                text.push(char::from_u32(braille).unwrap_or(' '));
                if any_lit {
                    let center_d = ((c as f32 / bw.max(1) as f32) - 0.5).abs() * 2.0;
                    let dim = 1.0 - center_d * 0.18;
                    let pulse = (frame as f32 * 0.12 + i as f32 * 1.4).sin() * 0.06 + 1.0;
                    let intensity = (norm_y * 0.55 + bands[i] * 0.35 + 0.10) * dim * pulse;
                    colors.push(pal(SPECTRUM, intensity.clamp(0.0, 1.0)));
                } else {
                    colors.push(Color::Reset);
                }
            }
            if i < bands.len() - 1 {
                text.push(' ');
                colors.push(Color::Reset);
            }
        }
        truncate_to_char_count(&mut text, width);
        colors.truncate(text.chars().count());
        lines.push(StyledVisLine { text, colors });
    }
    lines
}

// --- Columns: braille bars with per-sub-column interpolation ---

pub(super) fn render_columns_styled(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<StyledVisLine> {
    let total_sub_rows = height * 4;
    let num_bands = bands.len();
    let mut sub_energies: Vec<f32> = Vec::new();
    for b in 0..num_bands {
        let char_w = vis_band_width(b, width, num_bands);
        let sub_w = char_w * 2;
        let next = if b + 1 < num_bands { bands[b + 1] } else { bands[b] };
        for sc in 0..sub_w {
            let t = sc as f32 / sub_w.max(1) as f32;
            sub_energies.push((bands[b] * (1.0 - t) + next * t).clamp(0.0, 1.0));
        }
        if b < num_bands - 1 {
            sub_energies.push(0.0);
            sub_energies.push(0.0);
        }
    }
    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let norm_y = 1.0 - row as f32 / height.saturating_sub(1).max(1) as f32;
        let mut text = String::with_capacity(width);
        let mut colors = Vec::with_capacity(width);
        for cx in 0..width {
            let mut braille = 0x2800u32;
            let mut any_lit = false;
            let mut max_e = 0.0f32;
            for dc in 0..2usize {
                let sx = cx * 2 + dc;
                let energy = sub_energies.get(sx).copied().unwrap_or(0.0);
                max_e = max_e.max(energy);
                let fill = (energy * total_sub_rows as f32).round() as usize;
                let bar_top = total_sub_rows.saturating_sub(fill);
                for dy in 0..4 {
                    if fill > 0 && row * 4 + dy >= bar_top {
                        braille |= BRAILLE_BITS[dy][dc];
                        any_lit = true;
                    }
                }
            }
            text.push(char::from_u32(braille).unwrap_or(' '));
            if any_lit {
                let wave = (frame as f32 * 0.08 + cx as f32 * 0.15).sin() * 0.04;
                let intensity = (norm_y * 0.50 + max_e * 0.40 + 0.10 + wave).clamp(0.0, 1.0);
                colors.push(pal(SPECTRUM, intensity));
            } else {
                colors.push(Color::Reset);
            }
        }
        truncate_to_char_count(&mut text, width);
        colors.truncate(text.chars().count());
        lines.push(StyledVisLine { text, colors });
    }
    lines
}

// --- Wave: braille waveform with ocean coloring ---

pub(super) fn render_braille_wave_styled(
    samples: &[f32],
    width: usize,
    height: usize,
) -> Vec<StyledVisLine> {
    let dot_rows = height * 4;
    let dot_cols = width * 2;
    let center = dot_rows as f32 / 2.0;
    let mut ypos = vec![dot_rows / 2; dot_cols];
    if !samples.is_empty() {
        for (x, y) in ypos.iter_mut().enumerate() {
            let idx = x * samples.len() / dot_cols.max(1);
            let sample = samples[idx.min(samples.len() - 1)].clamp(-1.0, 1.0);
            *y = (((1.0 - sample) * (dot_rows.saturating_sub(1) as f32) / 2.0).round() as usize)
                .min(dot_rows.saturating_sub(1));
        }
    }
    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let mut text = String::with_capacity(width);
        let mut colors = Vec::with_capacity(width);
        for ch_col in 0..width {
            let mut braille = 0x2800u32;
            let mut max_amp = 0.0f32;
            for dc in 0..2 {
                let x = ch_col * 2 + dc;
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
                max_amp = max_amp.max((ypos[x] as f32 - center).abs() / center.max(1.0));
            }
            text.push(char::from_u32(braille).unwrap_or(' '));
            if braille != 0x2800 {
                let intensity = (max_amp * 0.70 + 0.30).clamp(0.0, 1.0);
                colors.push(pal(OCEAN, intensity));
            } else {
                colors.push(Color::Reset);
            }
        }
        lines.push(StyledVisLine { text, colors });
    }
    lines
}

// --- Scatter: braille particle scatter with sparkle ---

pub(super) fn render_braille_scatter_styled(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<StyledVisLine> {
    let dot_rows = (height * 4).max(1) as f32;
    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let mut line = String::with_capacity(width);
        let mut colors = Vec::with_capacity(width);
        for b in 0..bands.len() {
            let band_w = vis_band_width(b, width, bands.len());
            let energy = bands[b];
            let energy2 = energy * energy;
            let col_offset: usize =
                (0..b).map(|i| vis_band_width(i, width, bands.len()) + 1).sum();
            for c in 0..band_w {
                let mut braille = 0x2800u32;
                let mut lit = 0usize;
                for dr in 0..4 {
                    for dc in 0..2 {
                        let dot_y = row * 4 + dr;
                        let dot_x = c * 2 + dc;
                        let norm_y = 1.0 - dot_y as f32 / dot_rows;
                        let col_seed = (col_offset + c) as f32 * 1.37;
                        let rise_phase = norm_y * 4.0
                            + frame as f32 * (0.08 + energy * 0.14)
                            + col_seed;
                        let height_fade = 1.0 - norm_y * 0.75;
                        let spawn = energy2 * height_fade;
                        let drift = (rise_phase * 1.8 + b as f32 * 2.1).sin() * 0.25 * norm_y;
                        let drift_factor = 1.0 + drift.abs();
                        let threshold = spawn * drift_factor;
                        let h = scatter_hash(b, dot_y, dot_x, frame);
                        if h < threshold {
                            braille |= BRAILLE_BITS[dr][dc];
                            lit += 1;
                        }
                    }
                }
                line.push(char::from_u32(braille).unwrap_or(' '));

                let density = lit as f32 / 8.0;
                let norm_row_y = 1.0 - row as f32 / height.saturating_sub(1).max(1) as f32;
                let sparkle_val = scatter_hash(b, row, c, frame / 2);
                let sparkle_boost = if density > 0.0 && sparkle_val > 0.88 {
                    (sparkle_val - 0.88) * 3.0
                } else {
                    0.0
                };
                let wave_phase = frame as f32 * 0.045 + b as f32 * 0.55 + row as f32 * 0.11;
                let hue_wave = (wave_phase.sin() * 0.5 + 0.5) * energy * 0.08;
                let base_intensity =
                    density * 0.50 + energy * 0.28 + norm_row_y * 0.12 + sparkle_boost + hue_wave;
                let temporal_seed = scatter_hash(b, row, c, frame.saturating_sub(3) / 3);
                let intensity = (base_intensity * 0.84 + temporal_seed * 0.16)
                    .clamp(0.0, 1.0)
                    .powf(0.80);
                colors.push(pal(SPECTRUM, intensity));
            }
            if b < bands.len() - 1 {
                line.push(' ');
                colors.push(Color::Reset);
            }
        }
        truncate_to_char_count(&mut line, width);
        colors.truncate(line.chars().count());
        lines.push(StyledVisLine { text: line, colors });
    }
    lines
}

// --- Flame: braille fire animation ---

pub(super) fn render_braille_flame_styled(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<StyledVisLine> {
    let dot_rows = height * 4;
    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let mut text = String::with_capacity(width);
        let mut colors = Vec::with_capacity(width);
        for b in 0..bands.len() {
            let chars_per_band = vis_band_width(b, width, bands.len());
            let band_dot_cols = chars_per_band * 2;
            for c in 0..chars_per_band {
                let mut braille = 0x2800u32;
                let mut lit = 0usize;
                let mut max_depth = 0.0f32;
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
                                lit += 1;
                                max_depth = max_depth.max(1.0 - edge);
                            }
                        }
                    }
                }
                text.push(char::from_u32(braille).unwrap_or(' '));
                if lit > 0 {
                    let norm_y = 1.0 - row as f32 / height.saturating_sub(1).max(1) as f32;
                    let core = max_depth * 0.50 + (1.0 - norm_y) * 0.30 + bands[b] * 0.20;
                    colors.push(pal(FIRE, core.clamp(0.0, 1.0)));
                } else {
                    colors.push(Color::Reset);
                }
            }
            if b < bands.len() - 1 {
                text.push(' ');
                colors.push(Color::Reset);
            }
        }
        truncate_to_char_count(&mut text, width);
        colors.truncate(text.chars().count());
        lines.push(StyledVisLine { text, colors });
    }
    lines
}

// --- Matrix: falling katakana rain ---

pub(super) fn render_matrix_styled(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<StyledVisLine> {
    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let mut line = String::with_capacity(width);
        let mut colors = Vec::with_capacity(width);
        let mut col = 0usize;
        for (b, _) in bands.iter().enumerate() {
            let w = vis_band_width(b, width, bands.len());
            for _ in 0..w {
                let energy = bands[b];
                let seed = col as u64 * 7919 + 104_729;
                if scatter_hash(b, 0, col, frame / 20) > energy * 1.5 + 0.1 {
                    line.push(' ');
                    colors.push(Color::Reset);
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
                    colors.push(Color::Reset);
                } else {
                    let char_seed = seed ^ (row as u64 * 31 + (frame / 4) * 17);
                    line.push(MATRIX_CHARS[char_seed as usize % MATRIX_CHARS.len()]);
                    let intensity = if dist == 0 {
                        1.0
                    } else {
                        (1.0 - dist as f32 / trail_len.max(1) as f32).max(0.08)
                    };
                    colors.push(pal(MATRIX_PAL, intensity));
                }
                col += 1;
            }
            if b < bands.len() - 1 {
                line.push(' ');
                colors.push(Color::Reset);
                col += 1;
            }
        }
        truncate_to_char_count(&mut line, width);
        colors.truncate(line.chars().count());
        lines.push(StyledVisLine { text: line, colors });
    }
    lines
}

// --- Binary: scrolling 0/1 stream ---

pub(super) fn render_binary_styled(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<StyledVisLine> {
    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let mut line = String::with_capacity(width);
        let mut colors = Vec::with_capacity(width);
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
                let intensity = if bit_one && energy > 0.4 {
                    0.85 + energy * 0.15
                } else if bit_one || energy > 0.3 {
                    0.35 + energy * 0.25
                } else {
                    0.08 + energy * 0.12
                };
                colors.push(pal(CYBER, intensity.clamp(0.0, 1.0)));
                col += 1;
            }
            if b < bands.len() - 1 {
                line.push(' ');
                colors.push(Color::Reset);
                col += 1;
            }
        }
        truncate_to_char_count(&mut line, width);
        colors.truncate(line.chars().count());
        lines.push(StyledVisLine { text: line, colors });
    }
    lines
}

fn scatter_hash(band: usize, row: usize, col: usize, frame: u64) -> f32 {
    let f = (frame + (row * 3 + col) as u64) / 3;
    let mut h = band as u64 * 7919 + row as u64 * 6271 + col as u64 * 3037 + f * 104_729;
    h ^= h >> 16;
    h = h.wrapping_mul(0x45d9f3b37197344b);
    h ^= h >> 16;
    (h % 10_000) as f32 / 10_000.0
}
