use crate::render::weathr::HalfBlockCanvas;
use crate::render::weathr::types::RainIntensity;
use rand::prelude::*;
use ratatui::style::Color;

struct Raindrop {
    x: f32,
    y: f32,
    speed_y: f32,
    speed_x: f32,
    z_index: u8,
}

#[derive(Clone, Copy)]
struct Splash {
    x: f32,
    y: f32,
    timer: u8,
    max_timer: u8,
}

pub struct RaindropSystem {
    drops: Vec<Raindrop>,
    splashes: Vec<Splash>,
    terminal_width: u16,
    terminal_height: u16,
    intensity: RainIntensity,
    wind_x: f32,
}

impl RaindropSystem {
    pub fn new(terminal_width: u16, terminal_height: u16, intensity: RainIntensity) -> Self {
        let cap = match intensity {
            RainIntensity::Drizzle => (terminal_width / 4) as usize,
            RainIntensity::Light => (terminal_width / 2) as usize,
            RainIntensity::Heavy => terminal_width as usize,
            RainIntensity::Storm => (terminal_width as f32 * 1.5) as usize,
        };
        let mut system = Self {
            drops: Vec::with_capacity(cap),
            splashes: Vec::with_capacity(60),
            terminal_width,
            terminal_height,
            intensity,
            wind_x: 0.0,
        };
        let wind_dir = if rand::random::<bool>() { 1.0 } else { -1.0 };
        system.set_intensity_with_dir(intensity, wind_dir);
        system
    }

    pub fn set_intensity(&mut self, intensity: RainIntensity) {
        let current_dir = if self.wind_x >= 0.0 { 1.0 } else { -1.0 };
        self.set_intensity_with_dir(intensity, current_dir);
    }

    fn set_intensity_with_dir(&mut self, intensity: RainIntensity, dir: f32) {
        self.intensity = intensity;
        let base = match intensity {
            RainIntensity::Drizzle => 0.05,
            RainIntensity::Light => 0.1,
            RainIntensity::Heavy => 0.15,
            RainIntensity::Storm => 0.8,
        };
        self.wind_x = base * dir;
    }

    pub fn set_wind(&mut self, speed_kmh: f32, direction_deg: f32) {
        let factor = speed_kmh / 40.0;
        let rad = direction_deg.to_radians();
        self.wind_x = factor * (-rad.sin());
    }

    fn spawn_drop(&mut self, rng: &mut impl Rng) {
        let x = (rng.random::<u32>() % (self.terminal_width as u32 * 2)) as f32
            - (self.terminal_width as f32 * 0.5);
        let z = if rng.random::<bool>() { 1 } else { 0 };
        let speed_y = match self.intensity {
            RainIntensity::Drizzle => {
                if z == 1 {
                    0.4
                } else {
                    0.2
                }
            }
            RainIntensity::Light => {
                if z == 1 {
                    0.7
                } else {
                    0.4
                }
            }
            RainIntensity::Heavy => {
                if z == 1 {
                    0.9
                } else {
                    0.6
                }
            }
            RainIntensity::Storm => {
                if z == 1 {
                    1.8
                } else {
                    1.2
                }
            }
        };
        self.drops.push(Raindrop {
            x,
            y: 0.0,
            speed_y: speed_y + rng.random::<f32>() * 0.2,
            speed_x: self.wind_x + (rng.random::<f32>() * 0.1 - 0.05),
            z_index: z,
        });
    }

    pub fn update(&mut self, tw: u16, th: u16, rng: &mut impl Rng) {
        self.terminal_width = tw;
        self.terminal_height = th;
        let target = match self.intensity {
            RainIntensity::Drizzle => (tw / 4) as usize,
            RainIntensity::Light => (tw / 2) as usize,
            RainIntensity::Heavy => tw as usize,
            RainIntensity::Storm => (tw as f32 * 1.5) as usize,
        };
        if self.drops.len() < target {
            let rate = match self.intensity {
                RainIntensity::Drizzle => 1,
                RainIntensity::Light => 2,
                _ => 5,
            };
            for _ in 0..rate {
                self.spawn_drop(rng);
            }
        }
        let splash_chance = match self.intensity {
            RainIntensity::Drizzle => 0.1,
            RainIntensity::Light => 0.3,
            _ => 0.6,
        };
        let mut new_splashes = Vec::new();
        self.drops.retain_mut(|d| {
            d.y += d.speed_y;
            d.x += d.speed_x;
            if d.y >= (th - 1) as f32 {
                if d.z_index == 1 && rng.random::<f32>() < splash_chance {
                    new_splashes.push(Splash {
                        x: d.x,
                        y: (th - 1) as f32,
                        timer: 0,
                        max_timer: 3,
                    });
                }
                return false;
            }
            d.x > -10.0 && d.x < (tw as f32 + 10.0)
        });
        self.splashes.extend(new_splashes);
        self.splashes.retain_mut(|s| {
            s.timer += 1;
            s.timer < s.max_timer
        });
        if self.splashes.len() > 80 {
            self.splashes.drain(..self.splashes.len() - 80);
        }
    }

    pub fn render(&self, canvas: &mut HalfBlockCanvas, dark_bg: bool) {
        let (fg_near, fg_far) = if dark_bg {
            (Color::Rgb(160, 210, 255), Color::Rgb(80, 100, 130))
        } else {
            (Color::Rgb(40, 80, 140), Color::Rgb(100, 120, 150))
        };
        for drop in &self.drops {
            let color = if drop.z_index == 1 { fg_near } else { fg_far };
            let streak_len = match self.intensity {
                RainIntensity::Drizzle => 0.3,
                RainIntensity::Light => 0.6,
                RainIntensity::Heavy => 0.9,
                RainIntensity::Storm => 1.4,
            };
            let x0 = drop.x;
            let y0 = drop.y - streak_len;
            let x1 = drop.x + drop.speed_x * streak_len;
            let y1 = drop.y;
            canvas.draw_line(x0, y0, x1, y1, color);
        }
        let splash_color = if dark_bg {
            Color::Rgb(180, 220, 255)
        } else {
            Color::Rgb(60, 100, 160)
        };
        for splash in &self.splashes {
            let r = match splash.timer {
                0 => 0.2,
                1 => 0.4,
                _ => 0.6,
            };
            canvas.draw_circle(splash.x, splash.y, r, splash_color);
        }
    }
}
