use crate::render::weathr::TerminalRenderer;
use crate::render::weathr::types::FogIntensity;
use crossterm::style::Color;
use rand::prelude::*;
use std::collections::VecDeque;
use std::io;

struct FogBand {
    x: f32,
    y: f32,
    width: u8,
    speed_x: f32,
    density: f32,
    color: Color,
    lifetime: u32,
    max_lifetime: u32,
}

impl FogBand {
    fn new(terminal_width: u16, terminal_height: u16, intensity: FogIntensity, rng: &mut impl Rng) -> Self {
        let ground_level = terminal_height.saturating_sub(7);
        let fog_zone_depth = match intensity {
            FogIntensity::Light => 8,
            FogIntensity::Medium => 14,
            FogIntensity::Heavy => 20,
        };
        let fog_zone_top = ground_level.saturating_sub(fog_zone_depth);

        let y = fog_zone_top as f32 + (rng.random::<f32>() * fog_zone_depth as f32);
        let nearness = (y - fog_zone_top as f32) / fog_zone_depth as f32;
        let density = 0.3 + nearness * 0.5 + rng.random::<f32>() * 0.2;

        let base_width = match intensity {
            FogIntensity::Light => 4 + (rng.random::<u32>() % 6) as u8,
            FogIntensity::Medium => 6 + (rng.random::<u32>() % 10) as u8,
            FogIntensity::Heavy => 10 + (rng.random::<u32>() % 15) as u8,
        };

        let grey = 100 + (rng.random::<u32>() % 50) as u8;
        let color = Color::Rgb { r: grey, g: grey, b: grey + 8 };

        Self {
            x: rng.random::<f32>() * terminal_width as f32,
            y,
            width: base_width,
            speed_x: (rng.random::<f32>() - 0.5) * 0.12,
            density,
            color,
            lifetime: 0,
            max_lifetime: 150 + (rng.random::<u32>() % 250),
        }
    }

    fn update(&mut self) {
        self.x += self.speed_x;
        self.lifetime += 1;
    }

    fn is_alive(&self, terminal_width: u16) -> bool {
        self.lifetime < self.max_lifetime
            && self.x >= -(self.width as f32) - 5.0
            && self.x < (terminal_width as f32 + 5.0)
    }

    fn opacity(&self) -> f32 {
        let life_t = self.lifetime as f32 / self.max_lifetime as f32;
        let fade = if life_t < 0.15 {
            life_t / 0.15
        } else if life_t > 0.8 {
            (1.0 - life_t) / 0.2
        } else {
            1.0
        };
        self.density * fade.clamp(0.0, 1.0)
    }
}

pub struct FogSystem {
    bands: VecDeque<FogBand>,
    terminal_width: u16,
    terminal_height: u16,
    intensity: FogIntensity,
    spawn_timer: u32,
}

impl FogSystem {
    pub fn new(terminal_width: u16, terminal_height: u16, intensity: FogIntensity) -> Self {
        let cap = match intensity {
            FogIntensity::Light => (terminal_width as f32 * 0.3) as usize,
            FogIntensity::Medium => (terminal_width as f32 * 0.5) as usize,
            FogIntensity::Heavy => (terminal_width as f32 * 0.8) as usize,
        };

        Self {
            bands: VecDeque::with_capacity(cap),
            terminal_width,
            terminal_height,
            intensity,
            spawn_timer: 0,
        }
    }

    pub fn set_intensity(&mut self, intensity: FogIntensity) {
        self.intensity = intensity;
    }

    pub fn update(&mut self, terminal_width: u16, terminal_height: u16, rng: &mut impl Rng) {
        self.terminal_width = terminal_width;
        self.terminal_height = terminal_height;

        for band in &mut self.bands {
            band.update();
        }

        self.bands.retain(|b| b.is_alive(terminal_width));

        let (target_mult, spawn_delay) = match self.intensity {
            FogIntensity::Light => (0.25, 5),
            FogIntensity::Medium => (0.5, 3),
            FogIntensity::Heavy => (0.8, 1),
        };
        let target = (terminal_width as f32 * target_mult) as usize;

        self.spawn_timer += 1;
        if self.spawn_timer >= spawn_delay && self.bands.len() < target {
            self.spawn_timer = 0;
            self.bands.push_back(FogBand::new(terminal_width, terminal_height, self.intensity, rng));
        }
    }

    pub fn render(&self, renderer: &mut TerminalRenderer) -> io::Result<()> {
        for band in &self.bands {
            let opacity = band.opacity();
            if opacity < 0.1 {
                continue;
            }
            let y = band.y as i16;
            if y < 0 || y >= self.terminal_height as i16 {
                continue;
            }

            let ch = if opacity > 0.7 {
                '▒'
            } else if opacity > 0.4 {
                '░'
            } else {
                '~'
            };

            for dx in 0..band.width as i16 {
                let x = band.x as i16 + dx;
                if x >= 0 && x < self.terminal_width as i16 {
                    renderer.render_char(x as u16, y as u16, ch, band.color)?;
                }
            }
        }
        Ok(())
    }
}
