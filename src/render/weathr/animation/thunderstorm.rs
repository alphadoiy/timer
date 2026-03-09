use crate::render::weathr::TerminalRenderer;
use crossterm::style::Color;
use rand::prelude::*;
use std::collections::VecDeque;
use std::io;

const MAX_BOLTS: usize = 10;
const FLASH_GLOW_COLOR: Color = Color::Rgb { r: 180, g: 170, b: 220 };

#[derive(Clone, Copy, PartialEq)]
enum LightningState {
    Forming,
    Strike,
    Flash,
    Afterglow,
    Fading,
    Idle,
}

struct LightningBolt {
    segments: Vec<(u16, u16, char, bool)>,
    age: u8,
    max_age: u8,
}

pub struct ThunderstormSystem {
    bolts: VecDeque<LightningBolt>,
    state: LightningState,
    timer: u16,
    terminal_width: u16,
    terminal_height: u16,
    flash_active: bool,
    afterglow_active: bool,
    next_strike_in: u16,
}

impl ThunderstormSystem {
    pub fn new(terminal_width: u16, terminal_height: u16) -> Self {
        Self {
            bolts: VecDeque::with_capacity(MAX_BOLTS),
            state: LightningState::Idle,
            timer: 0,
            terminal_width,
            terminal_height,
            flash_active: false,
            afterglow_active: false,
            next_strike_in: 60 + (rand::random::<u16>() % 120),
        }
    }

    fn generate_bolt(&mut self, rng: &mut impl Rng) {
        let start_x = (rng.random::<u16>() % (self.terminal_width.saturating_sub(10))) + 5;
        let mut segments = Vec::new();
        let mut x = start_x as i16;
        let mut y: i16 = 2;

        segments.push((x as u16, y as u16, '⚡', false));

        while y < (self.terminal_height.saturating_sub(5)) as i16 {
            let direction = (rng.random::<i8>() % 3) - 1;
            x += direction as i16;
            y += 1;

            x = x.clamp(2, (self.terminal_width.saturating_sub(3)) as i16);

            let ch = match direction {
                -1 => '╱',
                1 => '╲',
                _ => '│',
            };

            segments.push((x as u16, y as u16, ch, false));

            if rng.random::<f32>() < 0.25 {
                let branch_dir: i16 = if rng.random::<bool>() { -1 } else { 1 };
                let branch_len = 2 + (rng.random::<u32>() % 5) as i16;
                let mut bx = x + branch_dir;
                let mut by = y + 1;
                for step in 0..branch_len {
                    if by >= (self.terminal_height.saturating_sub(2)) as i16 || bx < 1 || bx >= (self.terminal_width.saturating_sub(1)) as i16 {
                        break;
                    }
                    let jitter = if rng.random::<bool>() { branch_dir } else { 0 };
                    bx += jitter;
                    bx = bx.clamp(1, (self.terminal_width.saturating_sub(2)) as i16);
                    let bch = if jitter == branch_dir {
                        if branch_dir < 0 { '╱' } else { '╲' }
                    } else {
                        '│'
                    };
                    segments.push((bx as u16, by as u16, bch, true));
                    by += 1;

                    if step > 2 && rng.random::<f32>() < 0.4 {
                        break;
                    }
                }
            }
        }

        self.bolts.push_back(LightningBolt {
            segments,
            age: 0,
            max_age: 12,
        });

        while self.bolts.len() > MAX_BOLTS {
            self.bolts.pop_front();
        }
    }

    pub fn update(&mut self, terminal_width: u16, terminal_height: u16, rng: &mut impl Rng) {
        self.terminal_width = terminal_width;
        self.terminal_height = terminal_height;

        match self.state {
            LightningState::Idle => {
                self.flash_active = false;
                self.afterglow_active = false;
                if self.timer >= self.next_strike_in {
                    self.state = LightningState::Forming;
                    self.timer = 0;
                    self.generate_bolt(rng);
                } else {
                    self.timer += 1;
                }
            }
            LightningState::Forming => {
                self.state = LightningState::Strike;
                self.timer = 0;
            }
            LightningState::Strike => {
                self.flash_active = true;
                self.afterglow_active = false;
                self.state = LightningState::Flash;
                self.timer = 0;
            }
            LightningState::Flash => {
                self.flash_active = false;
                if self.timer > 2 {
                    self.afterglow_active = true;
                    self.state = LightningState::Afterglow;
                    self.timer = 0;
                } else {
                    self.timer += 1;
                }
            }
            LightningState::Afterglow => {
                if self.timer > 4 {
                    self.afterglow_active = false;
                    self.state = LightningState::Fading;
                    self.timer = 0;
                } else {
                    self.timer += 1;
                }
            }
            LightningState::Fading => {
                self.afterglow_active = false;
                self.bolts.retain_mut(|bolt| {
                    bolt.age += 1;
                    bolt.age < bolt.max_age
                });

                if self.bolts.is_empty() {
                    self.state = LightningState::Idle;
                    self.timer = 0;
                    self.next_strike_in = 30 + (rng.random::<u16>() % 200);
                }
            }
        }
    }

    pub fn is_flashing(&self) -> bool {
        self.flash_active
    }

    pub fn is_afterglow(&self) -> bool {
        self.afterglow_active
    }

    pub fn render(&self, renderer: &mut TerminalRenderer) -> io::Result<()> {
        for bolt in &self.bolts {
            let life_ratio = bolt.age as f32 / bolt.max_age as f32;

            for &(sx, sy, ch, is_branch) in &bolt.segments {
                let color = if self.flash_active {
                    Color::White
                } else if self.afterglow_active {
                    FLASH_GLOW_COLOR
                } else if is_branch {
                    if life_ratio > 0.5 {
                        Color::DarkGrey
                    } else {
                        Color::Rgb { r: 120, g: 110, b: 180 }
                    }
                } else if life_ratio > 0.6 {
                    Color::DarkYellow
                } else {
                    Color::Yellow
                };
                renderer.render_char(sx, sy, ch, color)?;
            }
        }

        if self.flash_active {
            let y = self.terminal_height.saturating_sub(2);
            for x in 0..self.terminal_width {
                renderer.render_char(x, y, '·', Color::White)?;
            }
        }
        Ok(())
    }
}
