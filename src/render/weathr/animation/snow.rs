use crate::render::weathr::HalfBlockCanvas;
use crate::render::weathr::types::SnowIntensity;
use rand::prelude::*;
use ratatui::style::Color;

const MAX_SNOW_DEPTH: u8 = 4;

struct Snowflake {
    x: f32,
    y: f32,
    speed_y: f32,
    speed_x: f32,
    sway_offset: f32,
    z_index: u8,
}

pub struct SnowSystem {
    flakes: Vec<Snowflake>,
    terminal_width: u16,
    terminal_height: u16,
    intensity: SnowIntensity,
    wind_x: f32,
    ground_snow: Vec<u8>,
}

impl SnowSystem {
    pub fn new(tw: u16, th: u16, intensity: SnowIntensity) -> Self {
        let cap = match intensity {
            SnowIntensity::Light => (tw / 4) as usize,
            SnowIntensity::Medium => (tw / 2) as usize,
            SnowIntensity::Heavy => tw as usize,
        };
        let mut sys = Self {
            flakes: Vec::with_capacity(cap),
            terminal_width: tw,
            terminal_height: th,
            intensity,
            wind_x: 0.0,
            ground_snow: vec![0; tw as usize],
        };
        let dir = if rand::random::<bool>() { 0.2 } else { -0.2 };
        sys.set_intensity_with_dir(intensity, dir);
        sys
    }

    pub fn set_intensity(&mut self, intensity: SnowIntensity) {
        let dir = if self.wind_x >= 0.0 { 1.0 } else { -1.0 };
        self.set_intensity_with_dir(intensity, dir);
    }

    fn set_intensity_with_dir(&mut self, intensity: SnowIntensity, dir: f32) {
        self.intensity = intensity;
        let base = match intensity {
            SnowIntensity::Light => 0.05,
            SnowIntensity::Medium => 0.1,
            SnowIntensity::Heavy => 0.2,
        };
        self.wind_x = base * dir;
    }

    pub fn set_wind(&mut self, speed_kmh: f32, direction_deg: f32) {
        let factor = speed_kmh / 20.0;
        let rad = direction_deg.to_radians();
        self.wind_x = factor * (-rad.sin());
    }

    fn spawn_flake(&mut self, rng: &mut impl Rng) {
        let x = (rng.random::<u32>() % (self.terminal_width as u32 * 3)) as f32
            - (self.terminal_width as f32);
        let z = if rng.random::<bool>() { 1 } else { 0 };
        let base_speed = match self.intensity {
            SnowIntensity::Light => {
                if z == 1 {
                    0.15
                } else {
                    0.08
                }
            }
            SnowIntensity::Medium => {
                if z == 1 {
                    0.2
                } else {
                    0.1
                }
            }
            SnowIntensity::Heavy => {
                if z == 1 {
                    0.3
                } else {
                    0.15
                }
            }
        };
        self.flakes.push(Snowflake {
            x,
            y: 0.0,
            speed_y: base_speed + rng.random::<f32>() * 0.05,
            speed_x: self.wind_x + (rng.random::<f32>() * 0.1 - 0.05),
            sway_offset: rng.random::<f32>() * 100.0,
            z_index: z,
        });
    }

    pub fn update(&mut self, tw: u16, th: u16, rng: &mut impl Rng) {
        self.terminal_width = tw;
        self.terminal_height = th;
        if self.ground_snow.len() != tw as usize {
            self.ground_snow.resize(tw as usize, 0);
        }
        let target = match self.intensity {
            SnowIntensity::Light => (tw / 4) as usize,
            SnowIntensity::Medium => (tw / 2) as usize,
            SnowIntensity::Heavy => tw as usize,
        };
        if self.flakes.len() < target {
            let rate = match self.intensity {
                SnowIntensity::Light => 1,
                SnowIntensity::Medium => 2,
                SnowIntensity::Heavy => 4,
            };
            for _ in 0..rate {
                self.spawn_flake(rng);
            }
        }
        let ground_snow = &mut self.ground_snow;
        self.flakes.retain_mut(|f| {
            f.y += f.speed_y;
            let sway = (f.y * 0.2 + f.sway_offset).sin() * 0.05;
            f.x += f.speed_x + sway;
            let col = f.x as usize;
            let depth = ground_snow.get(col).copied().unwrap_or(0);
            let land_y = (th.saturating_sub(1 + depth as u16)) as f32;
            if f.y >= land_y {
                if col < ground_snow.len() && ground_snow[col] < MAX_SNOW_DEPTH {
                    ground_snow[col] = ground_snow[col].saturating_add(1);
                }
                return false;
            }
            f.x > -20.0 && f.x < (tw as f32 + 20.0)
        });
    }

    pub fn render(&self, canvas: &mut HalfBlockCanvas, dark_bg: bool) {
        let (near, far) = if dark_bg {
            (Color::White, Color::Rgb(140, 140, 160))
        } else {
            (Color::Rgb(80, 80, 100), Color::Rgb(120, 120, 140))
        };
        for f in &self.flakes {
            let color = if f.z_index == 1 { near } else { far };
            canvas.plot_f(f.x, f.y, color);
            if f.z_index == 1 && self.intensity != SnowIntensity::Light {
                canvas.plot_f(f.x + 0.5, f.y, color);
            }
        }
        self.render_ground(canvas, dark_bg);
    }

    fn render_ground(&self, canvas: &mut HalfBlockCanvas, dark_bg: bool) {
        let (primary, secondary) = if dark_bg {
            (Color::White, Color::Rgb(200, 200, 220))
        } else {
            (Color::Rgb(160, 160, 180), Color::Rgb(120, 120, 140))
        };
        for (col, &depth) in self.ground_snow.iter().enumerate() {
            if depth == 0 {
                continue;
            }
            for d in 0..depth {
                let y = (self.terminal_height.saturating_sub(1 + d as u16)) as f32;
                let color = if d == 0 { primary } else { secondary };
                let density = if d == 0 { 0.8 } else { 0.5 };
                canvas.scatter_rect(col as f32, y, 1.0, 1.0, density, color, col as u32);
            }
        }
    }
}
