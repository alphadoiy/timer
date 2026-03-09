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
    tags: Vec<u8>,
}

pub(super) fn styled_from_plain(lines: Vec<String>, total_rows: usize) -> Vec<StyledVisLine> {
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
        0 => Color::Rgb(72, 188, 141),
        1 => Color::Rgb(92, 204, 160),
        2 => Color::Rgb(119, 218, 180),
        3 => Color::Rgb(154, 224, 162),
        4 => Color::Rgb(202, 228, 132),
        5 => Color::Rgb(240, 216, 109),
        6 => Color::Rgb(253, 186, 89),
        7 => Color::Rgb(255, 146, 98),
        _ => Color::Rgb(255, 102, 112),
    }
}

pub(super) fn styled_line_to_spans(line: &StyledVisLine) -> Line<'static> {
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

pub(super) fn render_bricks(
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

pub(super) fn render_columns(
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

pub(super) fn render_braille_wave(samples: &[f32], width: usize, height: usize) -> Vec<String> {
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

pub(super) fn render_braille_scatter_styled(
    bands: [f32; crate::music::NUM_BANDS],
    width: usize,
    height: usize,
    frame: u64,
) -> Vec<StyledVisLine> {
    let dot_rows = height * 4;
    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let mut line = String::with_capacity(width);
        let mut tags = Vec::with_capacity(width);
        let vertical = 1.0 - row as f32 / height.saturating_sub(1).max(1) as f32;
        for b in 0..bands.len() {
            let band_w = vis_band_width(b, width, bands.len());
            for c in 0..band_w {
                let mut braille = 0x2800u32;
                let mut lit = 0usize;
                for dr in 0..4 {
                    for dc in 0..2 {
                        let dot_row = row * 4 + dr;
                        let dot_col = c * 2 + dc;
                        let h = scatter_hash(b, dot_row, dot_col, frame);
                        let height_factor = 0.5 + 0.5 * (dot_row as f32 / dot_rows.max(1) as f32);
                        let threshold = bands[b] * bands[b] * height_factor;
                        if h < threshold {
                            braille |= BRAILLE_BITS[dr][dc];
                            lit += 1;
                        }
                    }
                }
                line.push(char::from_u32(braille).unwrap_or(' '));

                // Blend local dot density + band energy + slight row gradient.
                let density = lit as f32 / 8.0;
                let sparkle_now = scatter_hash(b, row, c, frame / 2);
                let sparkle_prev = scatter_hash(b, row, c, frame.saturating_sub(2) / 2);
                let sparkle_smooth = sparkle_now * 0.62 + sparkle_prev * 0.38;
                let sparkle_boost = if density > 0.0 && sparkle_smooth > 0.90 {
                    (sparkle_smooth - 0.90) * 2.0
                } else {
                    0.0
                };
                let wave_phase = frame as f32 * 0.045 + b as f32 * 0.55 + row as f32 * 0.11;
                let hue_wave = (wave_phase.sin() * 0.5 + 0.5) * bands[b] * 0.08;
                let base_intensity =
                    density * 0.55 + bands[b] * 0.30 + vertical * 0.10 + sparkle_boost + hue_wave;
                let temporal_seed = scatter_hash(b, row, c, frame.saturating_sub(3) / 3);
                let intensity = (base_intensity * 0.84 + temporal_seed * 0.16)
                    .clamp(0.0, 1.0)
                    .powf(0.80);
                tags.push(scatter_intensity_tag(intensity));
            }
            if b < bands.len() - 1 {
                line.push(' ');
                tags.push(0);
            }
        }
        truncate_to_char_count(&mut line, width);
        tags.truncate(line.chars().count());
        lines.push(StyledVisLine { text: line, tags });
    }
    lines
}

pub(super) fn render_braille_flame(
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

pub(super) fn render_matrix_styled(
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

pub(super) fn render_binary_styled(
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

fn scatter_intensity_tag(intensity: f32) -> u8 {
    if intensity < 0.10 {
        0
    } else if intensity < 0.19 {
        1
    } else if intensity < 0.29 {
        2
    } else if intensity < 0.40 {
        3
    } else if intensity < 0.53 {
        4
    } else if intensity < 0.66 {
        5
    } else if intensity < 0.77 {
        6
    } else if intensity < 0.88 {
        7
    } else {
        8
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
