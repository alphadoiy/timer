use crate::render::weathr::TerminalRenderer;
use crossterm::style::Color;
use rand::prelude::*;
use std::io;

const WING_COLORS: [(u8, u8, u8); 6] = [
    (255, 165, 0),   // Orange
    (255, 255, 100),  // Pale yellow
    (180, 100, 220),  // Lavender
    (100, 200, 255),  // Sky blue
    (255, 130, 130),  // Salmon
    (255, 255, 255),  // White
];

struct Butterfly {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    flap_phase: f32,
    flap_speed: f32,
    color: Color,
    drift_timer: u16,
}

impl Butterfly {
    fn new(terminal_width: u16, horizon_y: u16, rng: &mut impl Rng) -> Self {
        let x = rng.random::<f32>() * terminal_width as f32;
        let min_y = horizon_y.saturating_sub(12) as f32;
        let max_y = horizon_y.saturating_sub(2) as f32;
        let y = min_y + rng.random::<f32>() * (max_y - min_y).max(1.0);

        let c = WING_COLORS[(rng.random::<u32>() % WING_COLORS.len() as u32) as usize];
        let color = Color::Rgb { r: c.0, g: c.1, b: c.2 };

        Self {
            x,
            y,
            vx: (rng.random::<f32>() - 0.5) * 0.4,
            vy: (rng.random::<f32>() - 0.5) * 0.15,
            flap_phase: rng.random::<f32>() * std::f32::consts::TAU,
            flap_speed: 0.25 + rng.random::<f32>() * 0.15,
            color,
            drift_timer: 0,
        }
    }

    fn update(&mut self, terminal_width: u16, horizon_y: u16, rng: &mut impl Rng) {
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
        } else if self.x > terminal_width as f32 {
            self.x = terminal_width as f32;
            self.vx = -self.vx.abs();
        }

        let min_y = horizon_y.saturating_sub(12) as f32;
        let max_y = horizon_y.saturating_sub(2) as f32;
        self.y = self.y.clamp(min_y, max_y);
    }

    fn render(&self, renderer: &mut TerminalRenderer, tw: u16, th: u16) -> io::Result<()> {
        let x = self.x as i16;
        let y = self.y as i16;
        if y < 0 || y >= th as i16 {
            return Ok(());
        }

        let flap_val = self.flap_phase.sin();
        let (left, body, right) = if flap_val > 0.3 {
            ('╮', '•', '╭')
        } else if flap_val > -0.3 {
            ('─', '•', '─')
        } else {
            ('╯', '•', '╰')
        };

        if x > 0 && x - 1 < tw as i16 {
            renderer.render_char((x - 1) as u16, y as u16, left, self.color)?;
        }
        if x >= 0 && x < tw as i16 {
            renderer.render_char(x as u16, y as u16, body, Color::White)?;
        }
        if x + 1 >= 0 && x + 1 < tw as i16 {
            renderer.render_char((x + 1) as u16, y as u16, right, self.color)?;
        }
        Ok(())
    }
}

pub struct ButterflySystem {
    butterflies: Vec<Butterfly>,
    terminal_width: u16,
    terminal_height: u16,
}

impl ButterflySystem {
    pub fn new(terminal_width: u16, terminal_height: u16) -> Self {
        Self {
            butterflies: Vec::with_capacity(5),
            terminal_width,
            terminal_height,
        }
    }

    pub fn update(
        &mut self,
        terminal_width: u16,
        terminal_height: u16,
        horizon_y: u16,
        rng: &mut impl Rng,
    ) {
        self.terminal_width = terminal_width;
        self.terminal_height = terminal_height;

        for b in &mut self.butterflies {
            b.update(terminal_width, horizon_y, rng);
        }

        let max_count = 4.max(terminal_width / 25) as usize;
        if self.butterflies.len() < max_count && rng.random::<f32>() < 0.008 {
            self.butterflies.push(Butterfly::new(terminal_width, horizon_y, rng));
        }

        while self.butterflies.len() > max_count {
            self.butterflies.pop();
        }
    }

    pub fn render(&self, renderer: &mut TerminalRenderer) -> io::Result<()> {
        for b in &self.butterflies {
            b.render(renderer, self.terminal_width, self.terminal_height)?;
        }
        Ok(())
    }
}
