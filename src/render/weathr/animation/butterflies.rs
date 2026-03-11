use crate::render::weathr::HalfBlockCanvas;
use rand::prelude::*;
use ratatui::style::Color;

const WING_COLORS_DARK: [(u8, u8, u8); 6] = [
    (255, 165, 0),
    (255, 255, 100),
    (180, 100, 220),
    (100, 200, 255),
    (255, 130, 130),
    (255, 255, 255),
];

const WING_COLORS_LIGHT: [(u8, u8, u8); 6] = [
    (180, 100, 0),
    (160, 140, 0),
    (100, 40, 150),
    (0, 100, 180),
    (180, 60, 60),
    (80, 80, 80),
];

struct Butterfly {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    flap_phase: f32,
    flap_speed: f32,
    color_idx: usize,
    drift_timer: u16,
}

impl Butterfly {
    fn new(tw: u16, horizon_y: u16, rng: &mut impl Rng) -> Self {
        let x = rng.random::<f32>() * tw as f32;
        let min_y = horizon_y.saturating_sub(12) as f32;
        let max_y = horizon_y.saturating_sub(2) as f32;
        let y = min_y + rng.random::<f32>() * (max_y - min_y).max(1.0);
        let color_idx = (rng.random::<u32>() % WING_COLORS_DARK.len() as u32) as usize;
        Self {
            x,
            y,
            vx: (rng.random::<f32>() - 0.5) * 0.4,
            vy: (rng.random::<f32>() - 0.5) * 0.15,
            flap_phase: rng.random::<f32>() * std::f32::consts::TAU,
            flap_speed: 0.25 + rng.random::<f32>() * 0.15,
            color_idx,
            drift_timer: 0,
        }
    }

    fn update(&mut self, tw: u16, horizon_y: u16, rng: &mut impl Rng) {
        self.flap_phase += self.flap_speed;
        if self.flap_phase > std::f32::consts::TAU {
            self.flap_phase -= std::f32::consts::TAU;
        }
        self.x += self.vx;
        self.y += self.vy + (self.flap_phase * 0.5).sin() * 0.08;
        self.drift_timer += 1;
        if self.drift_timer > 40 + (rng.random::<u16>() % 60) {
            self.drift_timer = 0;
            self.vx = (rng.random::<f32>() - 0.5) * 0.4;
            self.vy = (rng.random::<f32>() - 0.5) * 0.15;
        }
        if self.x < 0.0 {
            self.x = 0.0;
            self.vx = self.vx.abs();
        } else if self.x > tw as f32 {
            self.x = tw as f32;
            self.vx = -self.vx.abs();
        }
        let min_y = horizon_y.saturating_sub(12) as f32;
        let max_y = horizon_y.saturating_sub(2) as f32;
        self.y = self.y.clamp(min_y, max_y);
    }
}

pub struct ButterflySystem {
    butterflies: Vec<Butterfly>,
    terminal_width: u16,
    terminal_height: u16,
}

impl ButterflySystem {
    pub fn new(tw: u16, th: u16) -> Self {
        Self {
            butterflies: Vec::with_capacity(5),
            terminal_width: tw,
            terminal_height: th,
        }
    }

    pub fn update(&mut self, tw: u16, th: u16, horizon_y: u16, rng: &mut impl Rng) {
        self.terminal_width = tw;
        self.terminal_height = th;
        for b in &mut self.butterflies {
            b.update(tw, horizon_y, rng);
        }
        let max_count = 4.max(tw / 25) as usize;
        if self.butterflies.len() < max_count && rng.random::<f32>() < 0.008 {
            self.butterflies.push(Butterfly::new(tw, horizon_y, rng));
        }
        while self.butterflies.len() > max_count {
            self.butterflies.pop();
        }
    }

    pub fn render(&self, canvas: &mut HalfBlockCanvas, dark_bg: bool) {
        let palette = if dark_bg {
            &WING_COLORS_DARK
        } else {
            &WING_COLORS_LIGHT
        };
        for b in &self.butterflies {
            let (r, g, bl) = palette[b.color_idx % palette.len()];
            let wing_color = Color::Rgb(r, g, bl);
            let body_color = if dark_bg {
                Color::White
            } else {
                Color::Rgb(40, 40, 40)
            };
            let flap = b.flap_phase.sin();
            let wing_dy = if flap > 0.3 {
                -0.3
            } else if flap > -0.3 {
                0.0
            } else {
                0.3
            };
            canvas.plot_f(b.x, b.y, body_color);
            canvas.plot_f(b.x - 0.5, b.y + wing_dy, wing_color);
            canvas.plot_f(b.x + 0.5, b.y + wing_dy, wing_color);
            canvas.plot_f(b.x - 1.0, b.y + wing_dy * 0.7, wing_color);
            canvas.plot_f(b.x + 1.0, b.y + wing_dy * 0.7, wing_color);
        }
    }
}
