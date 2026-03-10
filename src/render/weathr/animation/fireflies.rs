use crate::render::weathr::BrailleWeatherCanvas;
use rand::prelude::*;
use ratatui::style::Color;

struct Firefly {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    glow_phase: f32,
    glow_speed: f32,
    brightness: u8,
}

impl Firefly {
    fn new(tw: u16, horizon_y: u16, rng: &mut impl Rng) -> Self {
        let x = rng.random::<f32>() * tw as f32;
        let min_y = horizon_y.saturating_sub(8) as f32;
        let max_y = horizon_y.saturating_sub(1) as f32;
        let y = min_y + rng.random::<f32>() * (max_y - min_y);
        Self {
            x,
            y,
            vx: (rng.random::<f32>() - 0.5) * 0.3,
            vy: (rng.random::<f32>() - 0.5) * 0.2,
            glow_phase: rng.random::<f32>() * std::f32::consts::TAU,
            glow_speed: 0.1 + rng.random::<f32>() * 0.15,
            brightness: 0,
        }
    }

    fn update(&mut self, tw: u16, horizon_y: u16, rng: &mut impl Rng) {
        self.x += self.vx;
        self.y += self.vy;
        if rng.random::<f32>() < 0.02 {
            self.vx = (rng.random::<f32>() - 0.5) * 0.3;
            self.vy = (rng.random::<f32>() - 0.5) * 0.2;
        }
        if self.x < 0.0 {
            self.x = tw as f32;
        } else if self.x > tw as f32 {
            self.x = 0.0;
        }
        let min_y = horizon_y.saturating_sub(8) as f32;
        let max_y = horizon_y.saturating_sub(1) as f32;
        if self.y < min_y {
            self.y = min_y;
            self.vy = self.vy.abs();
        } else if self.y > max_y {
            self.y = max_y;
            self.vy = -self.vy.abs();
        }
        self.glow_phase += self.glow_speed;
        if self.glow_phase > std::f32::consts::TAU {
            self.glow_phase -= std::f32::consts::TAU;
        }
        let glow = (self.glow_phase.sin() + 1.0) / 2.0;
        self.brightness = (glow * 255.0) as u8;
    }
}

pub struct FireflySystem {
    fireflies: Vec<Firefly>,
    terminal_width: u16,
    terminal_height: u16,
}

impl FireflySystem {
    pub fn new(tw: u16, th: u16) -> Self {
        Self {
            fireflies: Vec::with_capacity((tw / 15).max(3) as usize),
            terminal_width: tw,
            terminal_height: th,
        }
    }

    pub fn update(&mut self, tw: u16, th: u16, horizon_y: u16, rng: &mut impl Rng) {
        self.terminal_width = tw;
        self.terminal_height = th;
        for f in &mut self.fireflies {
            f.update(tw, horizon_y, rng);
        }
        let target = (tw / 15).max(3) as usize;
        if self.fireflies.len() < target && rng.random::<f32>() < 0.01 {
            self.fireflies.push(Firefly::new(tw, horizon_y, rng));
        }
    }

    pub fn render_braille(&self, canvas: &mut BrailleWeatherCanvas, dark_bg: bool) {
        for f in &self.fireflies {
            if f.brightness <= 64 {
                continue;
            }
            let color = if f.brightness > 200 {
                if dark_bg { Color::Yellow } else { Color::Rgb(180, 160, 0) }
            } else if f.brightness > 128 {
                if dark_bg {
                    Color::Rgb(200, 255, 100)
                } else {
                    Color::Rgb(100, 140, 0)
                }
            } else if dark_bg {
                Color::Rgb(150, 200, 80)
            } else {
                Color::Rgb(80, 110, 40)
            };
            canvas.plot_f(f.x, f.y, color);
        }
    }
}
