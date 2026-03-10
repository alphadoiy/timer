use crate::render::weathr::BrailleWeatherCanvas;
use rand::prelude::*;
use ratatui::style::Color;

#[derive(Clone)]
struct Airplane {
    x: f32,
    y: f32,
    speed: f32,
}

pub struct AirplaneSystem {
    planes: Vec<Airplane>,
    terminal_width: u16,
    terminal_height: u16,
    spawn_cooldown: u16,
}

impl AirplaneSystem {
    pub fn new(tw: u16, th: u16) -> Self {
        Self {
            planes: Vec::with_capacity(2),
            terminal_width: tw,
            terminal_height: th,
            spawn_cooldown: 0,
        }
    }

    pub fn update(&mut self, tw: u16, th: u16, rng: &mut impl Rng) {
        self.terminal_width = tw;
        self.terminal_height = th;
        for plane in &mut self.planes {
            plane.x += plane.speed;
        }
        self.planes.retain(|p| p.x < tw as f32);
        self.spawn_cooldown = self.spawn_cooldown.saturating_sub(1);
        if self.spawn_cooldown == 0 && rng.random::<f32>() < 0.001 {
            let y = (rng.random::<u16>() % (th / 4)) as f32;
            let speed = 0.3 + rng.random::<f32>() * 0.2;
            self.planes.push(Airplane { x: 0.0, y, speed });
            self.spawn_cooldown = 600 + (rng.random::<u16>() % 300);
        }
    }

    pub fn render_braille(&self, canvas: &mut BrailleWeatherCanvas, dark_bg: bool) {
        let (body_color, wing_color, trail_color) = if dark_bg {
            (
                Color::White,
                Color::Rgb(180, 200, 220),
                Color::Rgb(100, 100, 120),
            )
        } else {
            (
                Color::Rgb(60, 60, 80),
                Color::Rgb(80, 90, 100),
                Color::Rgb(140, 140, 150),
            )
        };

        for plane in &self.planes {
            let x = plane.x;
            let y = plane.y;
            canvas.draw_line(x - 2.0, y, x + 3.0, y, body_color);
            canvas.draw_line(x - 0.5, y - 1.0, x + 1.0, y, wing_color);
            canvas.draw_line(x - 0.5, y + 1.0, x + 1.0, y, wing_color);
            canvas.draw_line(x + 2.0, y - 0.3, x + 3.0, y, wing_color);
            canvas.draw_line(x + 2.0, y + 0.3, x + 3.0, y, wing_color);
            canvas.plot_f(x - 3.0, y, trail_color);
            canvas.plot_f(x - 4.0, y, trail_color);
        }
    }
}
