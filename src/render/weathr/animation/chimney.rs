use crate::render::weathr::HalfBlockCanvas;
use rand::prelude::*;
use ratatui::style::Color;

const MAX_PARTICLES: usize = 200;
const MIN_MAX_AGE: u32 = 70;
const AGE_VARIANCE: u32 = 30;
const VERT_SPEED: f32 = 0.1;
const DRIFT_SCALE: f32 = 0.08;
const SPAWN_JITTER_X: f32 = 1.6;
const DEFAULT_SPAWN_RATE: u32 = 12;

struct SmokeParticle {
    x: f32,
    y: f32,
    age: u32,
    max_age: u32,
    drift: f32,
}

impl SmokeParticle {
    fn new(cx: u16, cy: u16, rng: &mut impl Rng) -> Self {
        Self {
            x: cx as f32 + (rng.random::<f32>() - 0.5) * SPAWN_JITTER_X,
            y: cy as f32,
            age: 0,
            max_age: MIN_MAX_AGE + (rng.random::<u32>() % AGE_VARIANCE),
            drift: (rng.random::<f32>() - 0.5) * DRIFT_SCALE,
        }
    }

    fn update(&mut self) {
        self.age += 1;
        self.y -= VERT_SPEED;
        self.x += self.drift;
    }

    fn is_alive(&self) -> bool {
        self.age < self.max_age
    }

    fn life_ratio(&self) -> f32 {
        self.age as f32 / self.max_age as f32
    }
}

pub struct ChimneySmoke {
    particles: Vec<SmokeParticle>,
    spawn_counter: u32,
    spawn_rate: u32,
}

impl ChimneySmoke {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(MAX_PARTICLES),
            spawn_counter: 0,
            spawn_rate: DEFAULT_SPAWN_RATE,
        }
    }

    pub fn update(&mut self, cx: u16, cy: u16, rng: &mut impl Rng) {
        for p in &mut self.particles {
            p.update();
        }
        self.particles.retain(|p| p.is_alive() && p.y >= 0.0);
        self.spawn_counter += 1;
        if self.spawn_counter >= self.spawn_rate && self.particles.len() < MAX_PARTICLES {
            self.spawn_counter = 0;
            self.particles.push(SmokeParticle::new(cx, cy, rng));
        }
    }

    pub fn render(&self, canvas: &mut HalfBlockCanvas, dark_bg: bool) {
        for p in &self.particles {
            let ratio = p.life_ratio();
            let color = if dark_bg {
                if ratio < 0.3 {
                    Color::White
                } else if ratio < 0.6 {
                    Color::Gray
                } else {
                    Color::DarkGray
                }
            } else if ratio < 0.3 {
                Color::Rgb(120, 120, 130)
            } else if ratio < 0.6 {
                Color::Rgb(140, 140, 150)
            } else {
                Color::Rgb(160, 160, 170)
            };
            canvas.plot_f(p.x, p.y, color);
        }
    }
}

impl Default for ChimneySmoke {
    fn default() -> Self {
        Self::new()
    }
}
