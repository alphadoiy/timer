use crate::render::weathr::HalfBlockCanvas;
use crate::render::weathr::types::FogIntensity;
use rand::prelude::*;
use ratatui::style::Color;
use std::collections::VecDeque;

struct FogBand {
    x: f32,
    y: f32,
    width: u8,
    speed_x: f32,
    density: f32,
    grey_level: u8,
    lifetime: u32,
    max_lifetime: u32,
}

impl FogBand {
    fn new(tw: u16, th: u16, intensity: FogIntensity, rng: &mut impl Rng) -> Self {
        let ground_level = th.saturating_sub(7);
        let fog_depth = match intensity {
            FogIntensity::Light => 8,
            FogIntensity::Medium => 14,
            FogIntensity::Heavy => 20,
        };
        let fog_top = ground_level.saturating_sub(fog_depth);
        let y = fog_top as f32 + rng.random::<f32>() * fog_depth as f32;
        let nearness = (y - fog_top as f32) / fog_depth as f32;
        let density = 0.3 + nearness * 0.5 + rng.random::<f32>() * 0.2;
        let base_width = match intensity {
            FogIntensity::Light => 4 + (rng.random::<u32>() % 6) as u8,
            FogIntensity::Medium => 6 + (rng.random::<u32>() % 10) as u8,
            FogIntensity::Heavy => 10 + (rng.random::<u32>() % 15) as u8,
        };
        let grey = 100 + (rng.random::<u32>() % 50) as u8;
        Self {
            x: rng.random::<f32>() * tw as f32,
            y,
            width: base_width,
            speed_x: (rng.random::<f32>() - 0.5) * 0.12,
            density,
            grey_level: grey,
            lifetime: 0,
            max_lifetime: 150 + (rng.random::<u32>() % 250),
        }
    }

    fn update(&mut self) {
        self.x += self.speed_x;
        self.lifetime += 1;
    }

    fn is_alive(&self, tw: u16) -> bool {
        self.lifetime < self.max_lifetime
            && self.x >= -(self.width as f32) - 5.0
            && self.x < (tw as f32 + 5.0)
    }

    fn opacity(&self) -> f32 {
        let t = self.lifetime as f32 / self.max_lifetime as f32;
        let fade = if t < 0.15 {
            t / 0.15
        } else if t > 0.8 {
            (1.0 - t) / 0.2
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
    pub fn new(tw: u16, th: u16, intensity: FogIntensity) -> Self {
        let cap = match intensity {
            FogIntensity::Light => (tw as f32 * 0.3) as usize,
            FogIntensity::Medium => (tw as f32 * 0.5) as usize,
            FogIntensity::Heavy => (tw as f32 * 0.8) as usize,
        };
        Self {
            bands: VecDeque::with_capacity(cap),
            terminal_width: tw,
            terminal_height: th,
            intensity,
            spawn_timer: 0,
        }
    }

    pub fn set_intensity(&mut self, intensity: FogIntensity) {
        self.intensity = intensity;
    }

    pub fn update(&mut self, tw: u16, th: u16, rng: &mut impl Rng) {
        self.terminal_width = tw;
        self.terminal_height = th;
        for band in &mut self.bands {
            band.update();
        }
        self.bands.retain(|b| b.is_alive(tw));
        let (target_mult, spawn_delay) = match self.intensity {
            FogIntensity::Light => (0.25, 5),
            FogIntensity::Medium => (0.5, 3),
            FogIntensity::Heavy => (0.8, 1),
        };
        let target = (tw as f32 * target_mult) as usize;
        self.spawn_timer += 1;
        if self.spawn_timer >= spawn_delay && self.bands.len() < target {
            self.spawn_timer = 0;
            self.bands
                .push_back(FogBand::new(tw, th, self.intensity, rng));
        }
    }

    pub fn render(&self, canvas: &mut HalfBlockCanvas, dark_bg: bool) {
        for band in &self.bands {
            let opacity = band.opacity();
            if opacity < 0.1 {
                continue;
            }
            let color = if dark_bg {
                let g = band.grey_level.min(200);
                Color::Rgb(g, g, g.saturating_add(8))
            } else {
                let g = 255u8.saturating_sub(band.grey_level).max(60);
                Color::Rgb(g, g, g.saturating_sub(5))
            };
            let density = opacity * 0.4;
            canvas.scatter_rect(
                band.x,
                band.y,
                band.width as f32,
                1.0,
                density,
                color,
                band.lifetime,
            );
            canvas.dither_ellipse(
                band.x + band.width as f32 * 0.5,
                band.y + 0.2,
                band.width as f32 * 0.7,
                0.8,
                (opacity * 0.22).clamp(0.08, 0.24),
                color,
                band.lifetime.wrapping_add(31),
            );
        }
    }
}
