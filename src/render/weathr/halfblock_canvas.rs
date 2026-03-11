use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};

#[derive(Clone, Copy, Default)]
struct CellPixels {
    top: Color,
    bottom: Color,
    overlay: Option<(char, Color)>,
}

/// A pixel canvas that renders using Unicode half-block characters (`▀`).
///
/// Each terminal character cell represents 2 vertical pixels.  The **top**
/// pixel is drawn with the cell's foreground colour and the **bottom** pixel
/// with the background colour, giving every cell two independently-coloured
/// "pixels".  This trades spatial resolution (1×2 vs Braille's 2×4) for
/// per-pixel colour fidelity — critical for clean pixel-art aesthetics.
///
/// The public API stays intentionally small and renderer-agnostic so scene
/// and animation modules do not encode backend-specific assumptions.
pub struct HalfBlockCanvas {
    area: Rect,
    width_cells: usize,
    height_cells: usize,
    /// Pixel-grid width  = width_cells × 1 (one pixel per cell column).
    pub sub_w: usize,
    /// Pixel-grid height = height_cells × 2 (two pixels per cell row).
    pub sub_h: usize,
    /// Per-cell pixel state, row-major, size = width_cells × height_cells.
    cells: Vec<CellPixels>,
}

impl HalfBlockCanvas {
    pub fn new(area: Rect) -> Self {
        let width_cells = area.width as usize;
        let height_cells = area.height as usize;
        let cell_count = width_cells.saturating_mul(height_cells);
        let sub_w = width_cells;
        let sub_h = height_cells.saturating_mul(2);
        Self {
            area,
            width_cells,
            height_cells,
            sub_w,
            sub_h,
            cells: vec![CellPixels::default(); cell_count],
        }
    }

    pub fn clear(&mut self) {
        self.cells.fill(CellPixels::default());
    }

    pub fn resize(&mut self, area: Rect) {
        let w = area.width as usize;
        let h = area.height as usize;
        if w != self.width_cells || h != self.height_cells {
            self.area = area;
            self.width_cells = w;
            self.height_cells = h;
            self.sub_w = w;
            self.sub_h = h.saturating_mul(2);
            let cell_count = w.saturating_mul(h);
            self.cells.resize(cell_count, CellPixels::default());
            self.clear();
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

    // ------------------------------------------------------------------
    // Plotting
    // ------------------------------------------------------------------

    /// Plot a pixel in sub-pixel coordinates.
    pub fn plot(&mut self, x_sub: i32, y_sub: i32, color: Color) {
        if x_sub < 0 || y_sub < 0 || x_sub >= self.sub_w as i32 || y_sub >= self.sub_h as i32 {
            return;
        }
        let cell_x = x_sub as usize;
        let cell_y = (y_sub as usize) / 2;
        let idx = cell_y * self.width_cells + cell_x;
        if let Some(cell) = self.cells.get_mut(idx) {
            if y_sub % 2 == 0 {
                cell.top = color;
            } else {
                cell.bottom = color;
            }
        }
    }

    /// Plot using float coordinates (cell units).
    /// Maps: x → x×1 sub-pixels, y → y×2 sub-pixels.
    pub fn plot_f(&mut self, x: f32, y: f32, color: Color) {
        self.plot((x * 1.0).round() as i32, (y * 2.0).round() as i32, color);
    }

    // ------------------------------------------------------------------
    // Drawing primitives
    // ------------------------------------------------------------------

    pub fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: Color) {
        let mut x0 = x0.round() as i32;
        let mut y0 = (y0 * 2.0).round() as i32;
        let x1 = x1.round() as i32;
        let y1 = (y1 * 2.0).round() as i32;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.plot(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = err * 2;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Draw circle outline that appears visually round in the terminal.
    pub fn draw_circle(&mut self, cx: f32, cy: f32, r: f32, color: Color) {
        let scx = cx;
        let scy = cy * 2.0;
        let srx = r;
        let sry = r * 2.0;
        let circumference = (2.0 * std::f32::consts::PI * srx.max(sry)) as usize;
        let steps = circumference.max(12);
        for i in 0..steps {
            let theta = (i as f32 / steps as f32) * std::f32::consts::TAU;
            let x = scx + theta.cos() * srx;
            let y = scy + theta.sin() * sry;
            self.plot(x.round() as i32, y.round() as i32, color);
        }
    }

    /// Fill a circle that appears visually round in the terminal.
    pub fn fill_circle(&mut self, cx: f32, cy: f32, r: f32, color: Color) {
        let scx = cx;
        let scy = cy * 2.0;
        let srx = r;
        let sry = r * 2.0;
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

    /// Fill an ellipse where `rx` and `ry` are radii in cell coordinates.
    pub fn fill_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, color: Color) {
        if ry == 0.0 {
            self.draw_line(cx - rx, cy, cx + rx, cy, color);
            return;
        }
        let scx = cx;
        let scy = cy * 2.0;
        let srx = rx;
        let sry = ry * 2.0;
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

    pub fn dither_ellipse(
        &mut self,
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        density: f32,
        color: Color,
        seed: u32,
    ) {
        if ry <= 0.0 || density <= 0.0 {
            return;
        }
        let scx = cx;
        let scy = cy * 2.0;
        let srx = rx.max(0.0);
        let sry = (ry * 2.0).max(0.0);
        let y_min = (scy - sry).floor() as i32;
        let y_max = (scy + sry).ceil() as i32;
        for sy in y_min..=y_max {
            let dy = sy as f32 - scy;
            let frac = dy / sry;
            if frac.abs() > 1.0 {
                continue;
            }
            let half_w = srx * (1.0 - frac * frac).sqrt();
            for sx in (scx - half_w).round() as i32..=(scx + half_w).round() as i32 {
                if (pseudo_hash(sx as u32, sy as u32, seed) % 1000) < (density * 1000.0) as u32 {
                    self.plot(sx, sy, color);
                }
            }
        }
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let sx = x.round() as i32;
        let sy = (y * 2.0).round() as i32;
        let sw = w.round() as i32;
        let sh = (h * 2.0).round() as i32;
        for dy in 0..sh {
            for dx in 0..sw {
                self.plot(sx + dx, sy + dy, color);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fill_triangle(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
    ) {
        let sx = [x0, x1, x2];
        let sy = [y0 * 2.0, y1 * 2.0, y2 * 2.0];
        let min_y = sy.iter().copied().reduce(f32::min).unwrap().floor() as i32;
        let max_y = sy.iter().copied().reduce(f32::max).unwrap().ceil() as i32;
        for row in min_y..=max_y {
            let mut x_ints = [0.0_f32; 3];
            let mut intersections = 0usize;
            for i in 0..3 {
                let j = (i + 1) % 3;
                let (ya, yb) = (sy[i], sy[j]);
                let (xa, xb) = (sx[i], sx[j]);
                if (ya <= row as f32 && yb > row as f32) || (yb <= row as f32 && ya > row as f32) {
                    let t = (row as f32 - ya) / (yb - ya);
                    x_ints[intersections] = xa + t * (xb - xa);
                    intersections += 1;
                }
            }
            if intersections >= 2 {
                if x_ints[0] > x_ints[1] {
                    x_ints.swap(0, 1);
                }
                if intersections == 3 {
                    if x_ints[1] > x_ints[2] {
                        x_ints.swap(1, 2);
                    }
                    if x_ints[0] > x_ints[1] {
                        x_ints.swap(0, 1);
                    }
                }
                let x_start = x_ints[0].round() as i32;
                let x_end = x_ints[intersections - 1].round() as i32;
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
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        density: f32,
        color: Color,
        seed: u32,
    ) {
        let sx = x.round() as i32;
        let sy = (y * 2.0).round() as i32;
        let sw = w.round() as i32;
        let sh = (h * 2.0).round() as i32;
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
                self.cells[idx].overlay = Some((ch, color));
            }
        }
    }

    // ------------------------------------------------------------------
    // Flush to ratatui Buffer
    // ------------------------------------------------------------------

    pub fn flush(&self, buf: &mut Buffer) {
        for y in 0..self.height_cells {
            for x in 0..self.width_cells {
                let ux = self.area.x + x as u16;
                let uy = self.area.y + y as u16;
                if ux >= buf.area().right() || uy >= buf.area().bottom() {
                    continue;
                }

                // Text overlay takes priority
                let cell_idx = y * self.width_cells + x;
                let cell = self.cells[cell_idx];
                if let Some((ch, color)) = cell.overlay {
                    let buf_cell = &mut buf[(ux, uy)];
                    buf_cell.reset();
                    buf_cell.set_char(ch).set_fg(color);
                    continue;
                }
                let top = cell.top;
                let bot = cell.bottom;

                let top_set = !matches!(top, Color::Reset);
                let bot_set = !matches!(bot, Color::Reset);

                if !top_set && !bot_set {
                    // Explicitly clear cells so content from prior frames cannot linger.
                    buf[(ux, uy)].reset();
                    continue;
                }

                let buf_cell = &mut buf[(ux, uy)];
                buf_cell.reset();
                if top_set && bot_set && top == bot {
                    // Both same colour → full block
                    buf_cell.set_symbol("█").set_style(Style::default().fg(top));
                } else if top_set && bot_set {
                    // Two different colours → upper half-block
                    buf_cell
                        .set_symbol("▀")
                        .set_style(Style::default().fg(top).bg(bot));
                } else if top_set {
                    buf_cell.set_symbol("▀").set_style(Style::default().fg(top));
                } else {
                    // only bottom set
                    buf_cell.set_symbol("▄").set_style(Style::default().fg(bot));
                }
            }
        }
    }
}

fn pseudo_hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut h = x
        .wrapping_mul(374761393)
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
        let mut canvas = HalfBlockCanvas::new(area);
        canvas.plot(5, 8, Color::White);
        let idx = 4 * 10 + 5;
        assert_eq!(canvas.cells[idx].top, Color::White);
    }

    #[test]
    fn plot_out_of_bounds_is_noop() {
        let area = Rect::new(0, 0, 10, 5);
        let mut canvas = HalfBlockCanvas::new(area);
        canvas.plot(-1, 0, Color::White);
        canvas.plot(10, 0, Color::White);
        canvas.plot(0, 10, Color::White);
        assert!(
            canvas
                .cells
                .iter()
                .all(|cell| matches!(cell.top, Color::Reset)
                    && matches!(cell.bottom, Color::Reset)
                    && cell.overlay.is_none())
        );
    }

    #[test]
    fn text_overlay_takes_priority() {
        let area = Rect::new(0, 0, 10, 5);
        let mut canvas = HalfBlockCanvas::new(area);
        canvas.plot(0, 0, Color::White);
        canvas.put_text(0, 0, "A", Color::Red);
        assert!(canvas.cells[0].overlay.is_some());
    }

    #[test]
    fn flush_produces_half_blocks() {
        let area = Rect::new(0, 0, 4, 2);
        let mut canvas = HalfBlockCanvas::new(area);
        // Plot top pixel red, bottom pixel blue in cell (0, 0)
        canvas.plot(0, 0, Color::Red); // top pixel of cell row 0
        canvas.plot(0, 1, Color::Blue); // bottom pixel of cell row 0
        let mut buffer = Buffer::empty(area);
        canvas.flush(&mut buffer);
        let cell = &buffer[(0, 0)];
        assert_eq!(cell.symbol(), "▀");
    }

    #[test]
    fn flush_clears_cells_after_canvas_clear() {
        let area = Rect::new(0, 0, 2, 1);
        let mut canvas = HalfBlockCanvas::new(area);
        let mut buffer = Buffer::empty(area);

        canvas.plot(0, 0, Color::Green);
        canvas.flush(&mut buffer);
        assert_eq!(buffer[(0, 0)].symbol(), "▀");

        canvas.clear();
        canvas.flush(&mut buffer);
        let cell = &buffer[(0, 0)];
        assert_eq!(cell.symbol(), " ");
        assert_eq!(cell.fg, Color::Reset);
        assert_eq!(cell.bg, Color::Reset);
    }

    #[test]
    fn draw_line_marks_endpoints() {
        let area = Rect::new(0, 0, 6, 3);
        let mut canvas = HalfBlockCanvas::new(area);
        canvas.draw_line(0.0, 0.0, 5.0, 2.0, Color::Yellow);

        assert_eq!(canvas.cells[0].top, Color::Yellow);
        let end_idx = 2 * 6 + 5;
        assert_eq!(canvas.cells[end_idx].top, Color::Yellow);
    }

    #[test]
    fn dither_ellipse_plots_pixels() {
        let area = Rect::new(0, 0, 8, 4);
        let mut canvas = HalfBlockCanvas::new(area);
        canvas.dither_ellipse(4.0, 2.0, 2.0, 1.0, 0.6, Color::Cyan, 7);
        assert!(
            canvas
                .cells
                .iter()
                .any(|cell| cell.top == Color::Cyan || cell.bottom == Color::Cyan)
        );
    }
}
