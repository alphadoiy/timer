use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};

pub struct BrailleWeatherCanvas {
    area: Rect,
    width_cells: usize,
    height_cells: usize,
    pub sub_w: usize,
    pub sub_h: usize,
    bits: Vec<u8>,
    colors: Vec<Color>,
    text_overlay: Vec<Option<(char, Color)>>,
}

impl BrailleWeatherCanvas {
    pub fn new(area: Rect) -> Self {
        let width_cells = area.width as usize;
        let height_cells = area.height as usize;
        let size = width_cells.saturating_mul(height_cells);
        Self {
            area,
            width_cells,
            height_cells,
            sub_w: width_cells.saturating_mul(2),
            sub_h: height_cells.saturating_mul(4),
            bits: vec![0; size],
            colors: vec![Color::Reset; size],
            text_overlay: vec![None; size],
        }
    }

    pub fn clear(&mut self) {
        self.bits.fill(0);
        self.colors.fill(Color::Reset);
        self.text_overlay.fill(None);
    }

    pub fn resize(&mut self, area: Rect) {
        let w = area.width as usize;
        let h = area.height as usize;
        if w != self.width_cells || h != self.height_cells {
            let size = w.saturating_mul(h);
            self.area = area;
            self.width_cells = w;
            self.height_cells = h;
            self.sub_w = w.saturating_mul(2);
            self.sub_h = h.saturating_mul(4);
            self.bits.resize(size, 0);
            self.colors.resize(size, Color::Reset);
            self.text_overlay.resize(size, None);
        } else {
            self.area = area;
        }
    }

    pub fn cell_width(&self) -> u16 {
        self.width_cells as u16
    }

    pub fn cell_height(&self) -> u16 {
        self.height_cells as u16
    }

    pub fn plot(&mut self, x_sub: i32, y_sub: i32, color: Color) {
        if x_sub < 0 || y_sub < 0 || x_sub >= self.sub_w as i32 || y_sub >= self.sub_h as i32 {
            return;
        }
        let cell_x = (x_sub / 2) as usize;
        let cell_y = (y_sub / 4) as usize;
        let sub_x = (x_sub % 2) as usize;
        let sub_y = (y_sub % 4) as usize;
        let idx = cell_y * self.width_cells + cell_x;
        if idx < self.bits.len() {
            self.bits[idx] |= braille_mask(sub_x, sub_y);
            self.colors[idx] = color;
        }
    }

    pub fn plot_f(&mut self, x: f32, y: f32, color: Color) {
        self.plot(
            (x * 2.0).round() as i32,
            (y * 4.0).round() as i32,
            color,
        );
    }

    pub fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: Color) {
        let sx0 = x0 * 2.0;
        let sy0 = y0 * 4.0;
        let sx1 = x1 * 2.0;
        let sy1 = y1 * 4.0;
        let dx = sx1 - sx0;
        let dy = sy1 - sy0;
        let steps = dx.abs().max(dy.abs()).max(1.0) as usize;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = sx0 + dx * t;
            let y = sy0 + dy * t;
            self.plot(x.round() as i32, y.round() as i32, color);
        }
    }

    pub fn draw_circle(&mut self, cx: f32, cy: f32, r: f32, color: Color) {
        let scx = cx * 2.0;
        let scy = cy * 4.0;
        let srx = r * 2.0;
        let sry = r * 4.0;
        let circumference = (2.0 * std::f32::consts::PI * srx.max(sry)) as usize;
        let steps = circumference.max(12);
        for i in 0..steps {
            let theta = (i as f32 / steps as f32) * std::f32::consts::TAU;
            let x = scx + theta.cos() * srx;
            let y = scy + theta.sin() * sry;
            self.plot(x.round() as i32, y.round() as i32, color);
        }
    }

    pub fn fill_circle(&mut self, cx: f32, cy: f32, r: f32, color: Color) {
        let scx = cx * 2.0;
        let scy = cy * 4.0;
        let srx = r * 2.0;
        let sry = r * 4.0;
        let y_min = (scy - sry).floor() as i32;
        let y_max = (scy + sry).ceil() as i32;
        for sy in y_min..=y_max {
            let dy = sy as f32 - scy;
            let frac = dy / sry;
            if frac.abs() > 1.0 {
                continue;
            }
            let half_w = srx * (1.0 - frac * frac).sqrt();
            let x_min = (scx - half_w).round() as i32;
            let x_max = (scx + half_w).round() as i32;
            for sx in x_min..=x_max {
                self.plot(sx, sy, color);
            }
        }
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let sx = (x * 2.0).round() as i32;
        let sy = (y * 4.0).round() as i32;
        let sw = (w * 2.0).round() as i32;
        let sh = (h * 4.0).round() as i32;
        for dy in 0..sh {
            for dx in 0..sw {
                self.plot(sx + dx, sy + dy, color);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fill_triangle(
        &mut self,
        x0: f32, y0: f32,
        x1: f32, y1: f32,
        x2: f32, y2: f32,
        color: Color,
    ) {
        let sx = [x0 * 2.0, x1 * 2.0, x2 * 2.0];
        let sy = [y0 * 4.0, y1 * 4.0, y2 * 4.0];
        let min_y = sy.iter().copied().reduce(f32::min).unwrap().floor() as i32;
        let max_y = sy.iter().copied().reduce(f32::max).unwrap().ceil() as i32;
        for row in min_y..=max_y {
            let mut x_ints = Vec::new();
            for i in 0..3 {
                let j = (i + 1) % 3;
                let (ya, yb) = (sy[i], sy[j]);
                let (xa, xb) = (sx[i], sx[j]);
                if (ya <= row as f32 && yb > row as f32)
                    || (yb <= row as f32 && ya > row as f32)
                {
                    let t = (row as f32 - ya) / (yb - ya);
                    x_ints.push(xa + t * (xb - xa));
                }
            }
            if x_ints.len() >= 2 {
                x_ints.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let x_start = x_ints[0].round() as i32;
                let x_end = x_ints[x_ints.len() - 1].round() as i32;
                for px in x_start..=x_end {
                    self.plot(px, row, color);
                }
            }
        }
    }

    /// Scatter dots in a rectangular region at the given density (0.0..1.0).
    #[allow(clippy::too_many_arguments)]
    pub fn scatter_rect(
        &mut self,
        x: f32, y: f32, w: f32, h: f32,
        density: f32,
        color: Color,
        seed: u32,
    ) {
        let sx = (x * 2.0).round() as i32;
        let sy = (y * 4.0).round() as i32;
        let sw = (w * 2.0).round() as i32;
        let sh = (h * 4.0).round() as i32;
        for dy in 0..sh {
            for dx in 0..sw {
                let px = sx + dx;
                let py = sy + dy;
                let hash = pseudo_hash(px as u32, py as u32, seed);
                if (hash % 1000) < (density * 1000.0) as u32 {
                    self.plot(px, py, color);
                }
            }
        }
    }

    pub fn put_text(&mut self, x_cell: u16, y_cell: u16, text: &str, color: Color) {
        for (i, ch) in text.chars().enumerate() {
            let cx = x_cell as usize + i;
            let cy = y_cell as usize;
            if cx < self.width_cells && cy < self.height_cells {
                let idx = cy * self.width_cells + cx;
                self.text_overlay[idx] = Some((ch, color));
            }
        }
    }

    pub fn flush(&self, buf: &mut Buffer) {
        for y in 0..self.height_cells {
            for x in 0..self.width_cells {
                let idx = y * self.width_cells + x;
                let ux = self.area.x + x as u16;
                let uy = self.area.y + y as u16;
                if ux >= buf.area().right() || uy >= buf.area().bottom() {
                    continue;
                }

                if let Some((ch, color)) = self.text_overlay[idx] {
                    buf[(ux, uy)]
                        .set_symbol(&ch.to_string())
                        .set_style(Style::default().fg(color));
                    continue;
                }

                let bits = self.bits[idx];
                if bits == 0 {
                    continue;
                }
                let ch = char::from_u32(0x2800 + bits as u32).unwrap_or(' ');
                buf[(ux, uy)]
                    .set_symbol(&ch.to_string())
                    .set_style(Style::default().fg(self.colors[idx]));
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

fn pseudo_hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut h = x.wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(seed.wrapping_mul(2246822519));
    h = (h ^ (h >> 13)).wrapping_mul(3266489917);
    h ^ (h >> 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plot_within_bounds() {
        let area = Rect::new(0, 0, 10, 5);
        let mut canvas = BrailleWeatherCanvas::new(area);
        canvas.plot(5, 8, Color::White);
        let idx = (8 / 4) * 10 + (5 / 2);
        assert_ne!(canvas.bits[idx], 0);
    }

    #[test]
    fn plot_out_of_bounds_is_noop() {
        let area = Rect::new(0, 0, 10, 5);
        let mut canvas = BrailleWeatherCanvas::new(area);
        canvas.plot(-1, 0, Color::White);
        canvas.plot(20, 0, Color::White);
        canvas.plot(0, 20, Color::White);
        assert!(canvas.bits.iter().all(|&b| b == 0));
    }

    #[test]
    fn text_overlay_takes_priority() {
        let area = Rect::new(0, 0, 10, 5);
        let mut canvas = BrailleWeatherCanvas::new(area);
        canvas.plot(0, 0, Color::White);
        canvas.put_text(0, 0, "A", Color::Red);
        assert!(canvas.text_overlay[0].is_some());
    }
}
