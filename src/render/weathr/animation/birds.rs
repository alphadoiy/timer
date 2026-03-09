use crate::render::weathr::TerminalRenderer;
use crossterm::style::Color;
use rand::prelude::*;
use std::io;

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
    pub fn new(terminal_width: u16, terminal_height: u16) -> Self {
        Self {
            flocks: Vec::with_capacity(2),
            terminal_width,
            terminal_height,
        }
    }

    fn spawn_flock(rng: &mut impl Rng, terminal_height: u16) -> Flock {
        let base_speed = 0.15 + (rng.random::<f32>() * 0.15);
        let base_y = (rng.random::<u16>() % (terminal_height / 3).max(1)) as f32;
        let follower_count = 2 + (rng.random::<u32>() % 4) as usize;

        let leader = Bird {
            x: 0.0,
            y: base_y,
            speed: base_speed,
            flap_phase: rng.random::<f32>() * std::f32::consts::TAU,
            flap_speed: 0.15 + rng.random::<f32>() * 0.08,
            bob_offset: 0.0,
        };

        let mut followers = Vec::with_capacity(follower_count);
        for i in 0..follower_count {
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

    pub fn update(&mut self, terminal_width: u16, terminal_height: u16, rng: &mut impl Rng) {
        self.terminal_width = terminal_width;
        self.terminal_height = terminal_height;

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
            let max_x = f.followers.iter()
                .map(|b| b.x)
                .fold(f.leader.x, f32::max);
            max_x < terminal_width as f32 + 5.0
        });

        if self.flocks.len() < 2 && rng.random::<f32>() < 0.003 {
            self.flocks.push(Self::spawn_flock(rng, terminal_height));
        }
    }

    pub fn render(&self, renderer: &mut TerminalRenderer) -> io::Result<()> {
        for flock in &self.flocks {
            render_bird(renderer, &flock.leader, self.terminal_width, self.terminal_height)?;
            for f in &flock.followers {
                render_bird(renderer, f, self.terminal_width, self.terminal_height)?;
            }
        }
        Ok(())
    }
}

fn render_bird(
    renderer: &mut TerminalRenderer,
    bird: &Bird,
    tw: u16,
    th: u16,
) -> io::Result<()> {
    let flap_val = bird.flap_phase.sin();
    let y = (bird.y + bird.bob_offset) as i16;
    let x = bird.x as i16;
    if y < 0 || y >= th as i16 {
        return Ok(());
    }

    let (left_wing, body, right_wing) = if flap_val > 0.3 {
        ('ˇ', 'w', 'ˇ')
    } else if flap_val > -0.3 {
        ('-', 'w', '-')
    } else {
        ('ˆ', 'w', 'ˆ')
    };

    let color = Color::Rgb { r: 60, g: 50, b: 40 };

    if x > 0 && x - 1 < tw as i16 {
        renderer.render_char((x - 1) as u16, y as u16, left_wing, color)?;
    }
    if x >= 0 && x < tw as i16 {
        renderer.render_char(x as u16, y as u16, body, color)?;
    }
    if x + 1 >= 0 && x + 1 < tw as i16 {
        renderer.render_char((x + 1) as u16, y as u16, right_wing, color)?;
    }
    Ok(())
}
