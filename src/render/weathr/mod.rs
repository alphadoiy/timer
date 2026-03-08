pub mod animation;
pub mod scene;
pub mod types;

use crossterm::style::Color;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Cell {
    character: char,
    color: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            character: ' ',
            color: Color::Reset,
        }
    }
}

pub struct TerminalRenderer {
    width: u16,
    height: u16,
    buffer: Vec<Cell>,
}

#[derive(Debug, Clone, Copy)]
pub struct ViewportTransform {
    pub scale: f32,
    pub offset_x: i16,
    pub offset_y: i16,
}

impl ViewportTransform {
    pub fn cover(logical_width: u16, logical_height: u16, area: Rect) -> Self {
        let x_scale = area.width as f32 / logical_width.max(1) as f32;
        let y_scale = area.height as f32 / logical_height.max(1) as f32;
        let scale = x_scale.max(y_scale).max(0.01);
        let scaled_w = (logical_width as f32 * scale).round() as i16;
        let scaled_h = (logical_height as f32 * scale).round() as i16;
        let offset_x = (area.width as i16 - scaled_w) / 2;
        let offset_y = (area.height as i16 - scaled_h) / 2;
        Self {
            scale,
            offset_x,
            offset_y,
        }
    }
}

impl TerminalRenderer {
    pub fn new(width: u16, height: u16) -> Self {
        let size = (width as usize).saturating_mul(height as usize);
        Self {
            width,
            height,
            buffer: vec![Cell::default(); size],
        }
    }

    pub fn clear(&mut self) {
        self.buffer.fill(Cell::default());
    }

    #[allow(dead_code)]
    pub fn render_centered_colored(
        &mut self,
        lines: &[String],
        start_row: u16,
        color: Color,
    ) -> std::io::Result<()> {
        let max_width = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let start_col = if self.width as usize > max_width {
            (self.width as usize - max_width) / 2
        } else {
            0
        };

        for (idx, line) in lines.iter().enumerate() {
            let row = start_row + idx as u16;
            if row < self.height {
                for (char_idx, ch) in line.chars().enumerate() {
                    let col = start_col as u16 + char_idx as u16;
                    if col < self.width {
                        let i = (row as usize) * (self.width as usize) + (col as usize);
                        if i < self.buffer.len() {
                            self.buffer[i] = Cell {
                                character: ch,
                                color,
                            };
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn render_line_colored(
        &mut self,
        x: u16,
        y: u16,
        text: &str,
        color: Color,
    ) -> std::io::Result<()> {
        if y >= self.height {
            return Ok(());
        }
        for (idx, ch) in text.chars().enumerate() {
            let col = x + idx as u16;
            if col < self.width {
                let i = (y as usize) * (self.width as usize) + (col as usize);
                if i < self.buffer.len() {
                    self.buffer[i] = Cell {
                        character: ch,
                        color,
                    };
                }
            }
        }
        Ok(())
    }

    pub fn render_char(&mut self, x: u16, y: u16, ch: char, color: Color) -> std::io::Result<()> {
        if x < self.width && y < self.height {
            let i = (y as usize) * (self.width as usize) + (x as usize);
            if i < self.buffer.len() {
                self.buffer[i] = Cell {
                    character: ch,
                    color,
                };
            }
        }
        Ok(())
    }

    pub fn flush_cover_to(&self, area: Rect, buf: &mut Buffer, viewport: ViewportTransform) {
        for dy in 0..area.height {
            for dx in 0..area.width {
                let sx_f = (dx as i16 - viewport.offset_x) as f32 / viewport.scale;
                let sy_f = (dy as i16 - viewport.offset_y) as f32 / viewport.scale;
                if sx_f < 0.0 || sy_f < 0.0 {
                    continue;
                }
                let sx = sx_f.floor() as u16;
                let sy = sy_f.floor() as u16;
                if sx >= self.width || sy >= self.height {
                    continue;
                }

                let i = (sy as usize) * (self.width as usize) + (sx as usize);
                if i >= self.buffer.len() {
                    continue;
                }
                let cell = self.buffer[i];
                if cell.character == ' ' {
                    continue;
                }
                super::put(
                    buf,
                    area.x as i16 + dx as i16,
                    area.y as i16 + dy as i16,
                    &cell.character.to_string(),
                    map_color(cell.color),
                );
            }
        }
    }

    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }
}

fn map_color(color: Color) -> ratatui::style::Color {
    match color {
        Color::Reset => ratatui::style::Color::Reset,
        Color::Black => ratatui::style::Color::Black,
        Color::DarkGrey => ratatui::style::Color::DarkGray,
        Color::Grey => ratatui::style::Color::Gray,
        Color::White => ratatui::style::Color::White,
        Color::Red | Color::DarkRed => ratatui::style::Color::Red,
        Color::Green | Color::DarkGreen => ratatui::style::Color::Green,
        Color::Yellow | Color::DarkYellow => ratatui::style::Color::Yellow,
        Color::Blue | Color::DarkBlue => ratatui::style::Color::Blue,
        Color::Cyan | Color::DarkCyan => ratatui::style::Color::Cyan,
        Color::Magenta | Color::DarkMagenta => ratatui::style::Color::Magenta,
        Color::Rgb { r, g, b } => ratatui::style::Color::Rgb(r, g, b),
        Color::AnsiValue(v) => ratatui::style::Color::Indexed(v),
    }
}

#[cfg(test)]
mod tests {
    use super::ViewportTransform;
    use ratatui::layout::Rect;

    #[test]
    fn viewport_cover_fills_area() {
        let area = Rect::new(0, 0, 80, 20);
        let viewport = ViewportTransform::cover(120, 34, area);
        let scaled_w = (120.0 * viewport.scale).round() as i16;
        let scaled_h = (34.0 * viewport.scale).round() as i16;
        assert!(scaled_w >= area.width as i16 || scaled_h >= area.height as i16);
    }
}
