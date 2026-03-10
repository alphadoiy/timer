use crate::render::weathr::BrailleWeatherCanvas;
use rand::prelude::*;
use ratatui::style::Color;

struct Cloud {
    x: f32,
    y: f32,
    speed: f32,
    wind_x: f32,
    rx: f32,
    ry: f32,
    bumps: Vec<(f32, f32, f32)>,
    is_dark: bool,
}

pub struct CloudSystem {
    clouds: Vec<Cloud>,
    terminal_width: u16,
    terminal_height: u16,
    base_wind_x: f32,
    safe_left: f32,
    safe_right: f32,
    safe_top: f32,
    safe_bottom: f32,
}

impl CloudSystem {
    pub fn new(tw: u16, th: u16) -> Self {
        let mut rng = rand::rng();
        let base_wind_x = 0.15;
        let count = (tw / 30).max(1) as usize;
        let segment = tw as f32 / count as f32;
        let mut clouds = Vec::with_capacity(count);
        for i in 0..count {
            let x_min = (i as f32 * segment) as u16;
            let x_max = ((i as f32 + 1.0) * segment) as u16;
            let x = rng.random_range(x_min..=x_max) as f32;
            clouds.push(Self::make_cloud(x, th, false, base_wind_x, &mut rng));
        }
        Self {
            clouds,
            terminal_width: tw,
            terminal_height: th,
            base_wind_x,
            safe_left: 0.0,
            safe_right: tw as f32,
            safe_top: 0.0,
            safe_bottom: th as f32,
        }
    }

    pub fn set_safe_area(&mut self, left: f32, right: f32, top: f32, bottom: f32) {
        self.safe_left = left.max(0.0);
        self.safe_right = right.min(self.terminal_width as f32).max(self.safe_left);
        self.safe_top = top.max(0.0);
        self.safe_bottom = bottom.min(self.terminal_height as f32).max(self.safe_top);
    }

    pub fn set_wind(&mut self, speed_kmh: f32, direction_deg: f32) {
        let rad = direction_deg.to_radians();
        self.base_wind_x = (speed_kmh / 50.0) * (-rad.sin());
        let mut rng = rand::rng();
        for c in &mut self.clouds {
            c.wind_x = self.base_wind_x * (0.8 + rng.random::<f32>() * 0.4);
        }
    }

    fn make_cloud(x: f32, th: u16, is_dark: bool, base_wind: f32, rng: &mut impl Rng) -> Cloud {
        let y_range = (th / 3).max(1);
        let y = rng.random_range(0..y_range) as f32;
        let speed = 0.02 + rng.random::<f32>() * 0.03;
        let wind_x = base_wind * (0.8 + rng.random::<f32>() * 0.4);
        let rx = 3.0 + rng.random::<f32>() * 4.0;
        let ry = 1.0 + rng.random::<f32>() * 1.5;
        let bump_count = 2 + (rng.random::<u32>() % 3) as usize;
        let mut bumps = Vec::with_capacity(bump_count);
        for _ in 0..bump_count {
            let bx = (rng.random::<f32>() - 0.5) * rx * 1.4;
            let by = -ry * (0.3 + rng.random::<f32>() * 0.5);
            let br = 1.0 + rng.random::<f32>() * 1.5;
            bumps.push((bx, by, br));
        }
        Cloud { x, y, speed, wind_x, rx, ry, bumps, is_dark }
    }

    pub fn update(&mut self, tw: u16, th: u16, is_clear: bool, rng: &mut impl Rng) {
        self.terminal_width = tw;
        self.terminal_height = th;
        for c in &mut self.clouds {
            c.x += c.speed + c.wind_x;
            let max_x = (self.safe_right - c.rx * 2.0).max(self.safe_left);
            c.x = c.x.clamp(self.safe_left, max_x);
            let max_y = (self.safe_bottom - c.ry * 2.0).max(self.safe_top);
            c.y = c.y.clamp(self.safe_top, max_y);
        }
        let max_clouds = if is_clear {
            (tw / 30) as usize
        } else {
            (tw / 20) as usize
        };
        let spawn_chance = if is_clear { 0.002 } else { 0.005 };
        let min_gap = (tw as f32 / 8.0).max(15.0);
        let too_close = self.clouds.iter().any(|c| c.x < self.safe_left + min_gap);
        if self.clouds.len() < max_clouds && !too_close && rng.random::<f32>() < spawn_chance {
            let mut cloud =
                Self::make_cloud(self.safe_left, th, !is_clear, self.base_wind_x, rng);
            let max_y = (self.safe_bottom - cloud.ry * 2.0).max(self.safe_top);
            cloud.y = cloud.y.clamp(self.safe_top, max_y);
            self.clouds.push(cloud);
        }
    }

    pub fn render_braille(&self, canvas: &mut BrailleWeatherCanvas, dark_bg: bool) {
        for cloud in &self.clouds {
            let body_color = if cloud.is_dark {
                Color::Rgb(100, 100, 110)
            } else if dark_bg {
                Color::Rgb(200, 200, 210)
            } else {
                Color::Rgb(80, 80, 90)
            };
            let edge_color = if cloud.is_dark {
                if dark_bg {
                    Color::Rgb(70, 70, 80)
                } else {
                    Color::Rgb(130, 130, 140)
                }
            } else if dark_bg {
                Color::Rgb(160, 160, 175)
            } else {
                Color::Rgb(110, 110, 120)
            };

            canvas.fill_circle(cloud.x, cloud.y, cloud.rx.min(cloud.ry), body_color);
            canvas.scatter_rect(
                cloud.x - cloud.rx,
                cloud.y - cloud.ry,
                cloud.rx * 2.0,
                cloud.ry * 2.0,
                0.6,
                body_color,
                (cloud.x * 7.0) as u32,
            );
            for &(bx, by, br) in &cloud.bumps {
                canvas.fill_circle(cloud.x + bx, cloud.y + by, br, body_color);
            }
            canvas.draw_circle(cloud.x, cloud.y, cloud.rx.min(cloud.ry) + 0.3, edge_color);
        }
    }
}
