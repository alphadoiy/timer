use super::*;

pub(crate) struct BrailleCanvas {
    area: Rect,
    width_cells: usize,
    height_cells: usize,
    pub(crate) width_sub: usize,
    pub(crate) height_sub: usize,
    bits: Vec<u8>,
    colors: Vec<ratatui::style::Color>,
}

impl BrailleCanvas {
    pub(crate) fn new(area: Rect) -> Self {
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
    pub(crate) fn plot(&mut self, x_sub: i32, y_sub: i32, color: ratatui::style::Color) {
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
    pub(crate) fn draw_line(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        color: ratatui::style::Color,
    ) {
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
    pub(crate) fn draw_ellipse(
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
    pub(crate) fn render(self, buf: &mut Buffer) {
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
