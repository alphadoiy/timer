use crate::render::weathr::BrailleWeatherCanvas;
use rand::prelude::*;
use ratatui::style::Color;

struct Bird {
    x: f32,
    y: f32,
    speed: f32,
    flap_phase: f32,
    flap_speed: f32,
    bob_offset: f32,
}

struct Flock {
    leader: Bird,
    followers: Vec<Bird>,
}

pub struct BirdSystem {
    flocks: Vec<Flock>,
    terminal_width: u16,
    terminal_height: u16,
}

impl BirdSystem {
    pub fn new(tw: u16, th: u16) -> Self {
        Self {
            flocks: Vec::with_capacity(2),
            terminal_width: tw,
            terminal_height: th,
        }
    }

    fn spawn_flock(rng: &mut impl Rng, th: u16) -> Flock {
        let base_speed = 0.15 + rng.random::<f32>() * 0.15;
        let base_y = (rng.random::<u16>() % (th / 3).max(1)) as f32;
        let count = 2 + (rng.random::<u32>() % 4) as usize;
        let leader = Bird {
            x: 0.0,
            y: base_y,
            speed: base_speed,
            flap_phase: rng.random::<f32>() * std::f32::consts::TAU,
            flap_speed: 0.15 + rng.random::<f32>() * 0.08,
            bob_offset: 0.0,
        };
        let mut followers = Vec::with_capacity(count);
        for i in 0..count {
            let side = if i % 2 == 0 { 1.0 } else { -1.0 };
            let rank = ((i / 2) + 1) as f32;
            followers.push(Bird {
                x: -(rank * 3.0 + rng.random::<f32>() * 0.5),
                y: base_y + side * rank * 1.2,
                speed: base_speed + (rng.random::<f32>() - 0.5) * 0.02,
                flap_phase: rng.random::<f32>() * std::f32::consts::TAU,
                flap_speed: 0.15 + rng.random::<f32>() * 0.08,
                bob_offset: rng.random::<f32>() * std::f32::consts::TAU,
            });
        }
        Flock { leader, followers }
    }

    pub fn update(&mut self, tw: u16, th: u16, rng: &mut impl Rng) {
        self.terminal_width = tw;
        self.terminal_height = th;
        for flock in &mut self.flocks {
            flock.leader.x += flock.leader.speed;
            flock.leader.flap_phase += flock.leader.flap_speed;
            flock.leader.bob_offset = (flock.leader.flap_phase * 0.3).sin() * 0.2;
            for f in &mut flock.followers {
                f.x += f.speed;
                f.flap_phase += f.flap_speed;
                f.bob_offset = (f.flap_phase * 0.3).sin() * 0.2;
            }
        }
        self.flocks.retain(|f| {
            let max_x = f.followers.iter().map(|b| b.x).fold(f.leader.x, f32::max);
            max_x < tw as f32 + 5.0
        });
        if self.flocks.len() < 2 && rng.random::<f32>() < 0.003 {
            self.flocks.push(Self::spawn_flock(rng, th));
        }
    }

    pub fn render_braille(&self, canvas: &mut BrailleWeatherCanvas, dark_bg: bool) {
        let color = if dark_bg {
            Color::Rgb(166, 176, 194)
        } else {
            Color::Rgb(72, 82, 96)
        };
        for flock in &self.flocks {
            render_bird_braille(canvas, &flock.leader, color);
            for f in &flock.followers {
                render_bird_braille(canvas, f, color);
            }
        }
    }
}

fn render_bird_braille(canvas: &mut BrailleWeatherCanvas, bird: &Bird, color: Color) {
    let x = bird.x;
    let y = bird.y + bird.bob_offset;
    let flap = bird.flap_phase.sin();
    let wing_dy = if flap > 0.3 {
        -0.3
    } else if flap > -0.3 {
        0.0
    } else {
        0.3
    };
    canvas.plot_f(x, y, color);
    canvas.plot_f(x - 0.5, y + wing_dy, color);
    canvas.plot_f(x + 0.5, y + wing_dy, color);
    canvas.plot_f(x - 1.0, y + wing_dy * 1.5, color);
    canvas.plot_f(x + 1.0, y + wing_dy * 1.5, color);
}
