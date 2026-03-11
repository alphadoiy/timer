use crate::render::weathr::HalfBlockCanvas;
use rand::prelude::*;
use ratatui::style::Color;

#[derive(Clone, Copy)]
struct Star {
    x: u16,
    y: u16,
    brightness: f32,
    phase: f32,
}

struct ShootingStar {
    x: f32,
    y: f32,
    speed_x: f32,
    speed_y: f32,
    length: usize,
    active: bool,
}

pub struct StarSystem {
    stars: Vec<Star>,
    shooting_star: Option<ShootingStar>,
    terminal_width: u16,
    terminal_height: u16,
}

impl StarSystem {
    const MIN_DISTANCE: f32 = 3.0;

    pub fn new(tw: u16, th: u16) -> Self {
        let stars = Self::create_stars(tw, th, &[]);
        Self {
            stars,
            shooting_star: None,
            terminal_width: tw,
            terminal_height: th,
        }
    }

    fn create_stars(tw: u16, th: u16, initial: &[Star]) -> Vec<Star> {
        let mut rng = rand::rng();
        let count = (tw as usize * th as usize) / 80;
        let mut stars: Vec<Star> = initial
            .iter()
            .copied()
            .filter(|s| s.x < tw && s.y < th / 2)
            .take(count)
            .collect();
        let needed = count.saturating_sub(stars.len());
        for _ in 0..needed {
            let mut attempts = 0;
            loop {
                let x = rng.random::<u16>() % tw;
                let y = rng.random::<u16>() % (th / 2);
                let too_close = stars.iter().any(|s| {
                    let dx = (s.x as f32 - x as f32).abs();
                    let dy = (s.y as f32 - y as f32).abs();
                    (dx * dx + dy * dy).sqrt() < Self::MIN_DISTANCE
                });
                if !too_close || attempts >= 50 {
                    stars.push(Star {
                        x,
                        y,
                        brightness: rng.random::<f32>(),
                        phase: rng.random::<f32>() * std::f32::consts::TAU,
                    });
                    break;
                }
                attempts += 1;
            }
        }
        stars
    }

    pub fn update(&mut self, tw: u16, th: u16, rng: &mut impl Rng) {
        if tw != self.terminal_width || th != self.terminal_height {
            self.stars = Self::create_stars(tw, th, &self.stars);
            self.terminal_width = tw;
            self.terminal_height = th;
            return;
        }
        for star in &mut self.stars {
            star.phase += 0.05;
            star.brightness = (star.phase.sin() + 1.0) / 2.0;
        }
        if let Some(ref mut s) = self.shooting_star {
            s.x += s.speed_x;
            s.y += s.speed_y;
            if s.x < 0.0 || s.y as u16 >= th || s.length == 0 {
                self.shooting_star = None;
            }
        } else if rng.random::<f32>() < 0.005 {
            let sx = (rng.random::<u16>() % (tw / 2)) + (tw / 4);
            let sy = rng.random::<u16>() % (th / 4);
            self.shooting_star = Some(ShootingStar {
                x: sx as f32,
                y: sy as f32,
                speed_x: if rng.random::<bool>() { 1.5 } else { -1.5 },
                speed_y: 0.5 + rng.random::<f32>() * 0.5,
                length: 5,
                active: true,
            });
        }
    }

    pub fn render(&self, canvas: &mut HalfBlockCanvas, dark_bg: bool) {
        for star in &self.stars {
            let color = if star.brightness > 0.6 {
                if dark_bg {
                    Color::White
                } else {
                    Color::Rgb(60, 60, 80)
                }
            } else if dark_bg {
                Color::Rgb(100, 100, 120)
            } else {
                Color::Rgb(120, 120, 140)
            };
            if star.brightness > 0.3 {
                canvas.plot_f(star.x as f32, star.y as f32, color);
            }
        }
        if let Some(ref s) = self.shooting_star {
            if !s.active {
                return;
            }
            let head_color = if dark_bg {
                Color::White
            } else {
                Color::Rgb(40, 40, 60)
            };
            let tail_color = if dark_bg {
                Color::Rgb(160, 160, 180)
            } else {
                Color::Rgb(100, 100, 120)
            };
            canvas.plot_f(s.x, s.y, head_color);
            for i in 1..s.length {
                let tx = s.x - s.speed_x * i as f32;
                let ty = s.y - s.speed_y * i as f32;
                canvas.plot_f(tx, ty, tail_color);
            }
        }
    }
}
