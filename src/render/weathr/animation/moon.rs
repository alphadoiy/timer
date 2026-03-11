use crate::render::weathr::HalfBlockCanvas;
use ratatui::style::Color;

const MOON_SPRITE: &[&str] = &[
    "...EEE...",
    "..EBBBE..",
    ".EBBHBBE.",
    ".BBBCBBE.",
    "EBBBBBBBE",
    ".EBBCBBE.",
    ".EBBBBHE.",
    "..EBBBE..",
    "...EEE...",
];

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

    pub fn render(&self, canvas: &mut HalfBlockCanvas, dark_bg: bool) {
        let cx = self.x as f32;
        let cy = self.y as f32;

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

        render_moon_sprite(canvas, cx, cy, body_color, crater_color, edge_color);
        canvas.dither_ellipse(cx - 0.8, cy - 0.6, 2.2, 0.8, 0.35, edge_color, 41);

        match illum {
            1 | 7 => {
                let lit_side = if illum == 1 { 1.0 } else { -1.0 };
                apply_moon_shadow(canvas, cx, cy, lit_side, 0.72, 0.18, body_color, 73);
            }
            2 | 6 => {
                let lit_side = if illum == 2 { 1.0 } else { -1.0 };
                apply_moon_shadow(canvas, cx, cy, lit_side, 0.45, 0.2, body_color, 97);
            }
            3 | 5 => {}
            _ => {}
        }
    }
}

fn render_moon_sprite(
    canvas: &mut HalfBlockCanvas,
    cx: f32,
    cy: f32,
    body_color: Color,
    crater_color: Color,
    edge_color: Color,
) {
    let base_x = cx.round() as i32 - (MOON_SPRITE[0].len() as i32 / 2);
    let base_y = (cy * 2.0).round() as i32 - (MOON_SPRITE.len() as i32 / 2);
    for (row, line) in MOON_SPRITE.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            let color = match ch {
                'B' => Some(body_color),
                'C' => Some(crater_color),
                'E' => Some(edge_color),
                'H' => Some(edge_color),
                _ => None,
            };
            if let Some(color) = color {
                canvas.plot(base_x + col as i32, base_y + row as i32, color);
            }
        }
    }
}

fn apply_moon_shadow(
    canvas: &mut HalfBlockCanvas,
    cx: f32,
    cy: f32,
    lit_side: f32,
    offset: f32,
    density: f32,
    rim_color: Color,
    seed: u32,
) {
    let shadow_cx = cx - lit_side * offset;
    canvas.fill_circle(shadow_cx, cy, 2.8, Color::Reset);
    canvas.dither_ellipse(shadow_cx, cy, 2.2, 2.8, density, rim_color, seed);
}
