use crate::render::weathr::BrailleWeatherCanvas;
use ratatui::style::Color;

pub struct MoonSystem {
    phase: f64,
    x: u16,
    y: u16,
}

impl MoonSystem {
    pub fn new(tw: u16, th: u16, phase: Option<f64>) -> Self {
        Self {
            phase: phase.unwrap_or(0.5),
            x: (tw * 5 / 6).min(tw.saturating_sub(8)),
            y: (th / 4).max(2),
        }
    }

    pub fn set_phase(&mut self, phase: f64) {
        self.phase = phase;
    }

    pub fn update(&mut self, tw: u16, th: u16) {
        self.x = (tw * 5 / 6).min(tw.saturating_sub(8));
        self.y = (th / 4).max(2);
    }

    pub fn set_position(&mut self, x: u16, y: u16) {
        self.x = x;
        self.y = y;
    }

    pub fn render_braille(&self, canvas: &mut BrailleWeatherCanvas, dark_bg: bool) {
        let cx = self.x as f32;
        let cy = self.y as f32;
        let r = 3.5;

        let (body_color, crater_color, edge_color) = if dark_bg {
            (
                Color::Rgb(230, 215, 140),
                Color::Rgb(170, 155, 95),
                Color::Rgb(240, 230, 170),
            )
        } else {
            (
                Color::Rgb(140, 125, 60),
                Color::Rgb(100, 90, 50),
                Color::Rgb(160, 145, 80),
            )
        };

        let illum = (self.phase * 8.0).round() as usize % 8;
        if illum == 0 {
            return;
        }

        canvas.fill_circle(cx, cy, r, body_color);
        canvas.draw_circle(cx, cy, r, edge_color);

        match illum {
            1 | 7 => {
                let shadow_cx = if illum == 1 {
                    cx - r * 0.6
                } else {
                    cx + r * 0.6
                };
                canvas.fill_circle(shadow_cx, cy, r * 0.85, Color::Reset);
            }
            2 | 6 => {
                let shadow_cx = if illum == 2 {
                    cx - r * 0.3
                } else {
                    cx + r * 0.3
                };
                canvas.fill_circle(shadow_cx, cy, r * 0.7, Color::Reset);
            }
            3 | 5 => {}
            4 => {
                canvas.plot_f(cx - 0.8, cy - 0.5, crater_color);
                canvas.plot_f(cx + 0.5, cy + 0.3, crater_color);
                canvas.plot_f(cx - 0.2, cy + 0.8, crater_color);
            }
            _ => {}
        }

        if (3..=5).contains(&illum) {
            canvas.plot_f(cx - 0.8, cy - 0.5, crater_color);
            canvas.plot_f(cx + 0.5, cy + 0.3, crater_color);
            canvas.plot_f(cx - 0.2, cy + 0.8, crater_color);
        }
    }
}
