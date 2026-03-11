use crate::render::weathr::HalfBlockCanvas;
use rand::prelude::*;
use ratatui::style::Color;
use std::collections::VecDeque;

const MAX_BOLTS: usize = 10;

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
    segments: Vec<(f32, f32, f32, f32, bool)>,
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
    pub fn new(tw: u16, th: u16) -> Self {
        Self {
            bolts: VecDeque::with_capacity(MAX_BOLTS),
            state: LightningState::Idle,
            timer: 0,
            terminal_width: tw,
            terminal_height: th,
            flash_active: false,
            afterglow_active: false,
            next_strike_in: 60 + (rand::random::<u16>() % 120),
        }
    }

    fn generate_bolt(&mut self, rng: &mut impl Rng) {
        let start_x = (rng.random::<u16>() % (self.terminal_width.saturating_sub(10))) + 5;
        let mut segments = Vec::new();
        let mut x = start_x as f32;
        let mut y: f32 = 2.0;

        while y < (self.terminal_height.saturating_sub(5)) as f32 {
            let direction = (rng.random::<i8>() % 3) as f32 - 1.0;
            let nx = (x + direction).clamp(2.0, (self.terminal_width.saturating_sub(3)) as f32);
            let ny = y + 1.0;
            segments.push((x, y, nx, ny, false));
            x = nx;
            y = ny;

            if rng.random::<f32>() < 0.25 {
                let branch_dir: f32 = if rng.random::<bool>() { -1.0 } else { 1.0 };
                let branch_len = 2 + (rng.random::<u32>() % 5) as usize;
                let mut bx = x + branch_dir;
                let mut by = y + 1.0;
                for step in 0..branch_len {
                    if by >= (self.terminal_height.saturating_sub(2)) as f32
                        || bx < 1.0
                        || bx >= (self.terminal_width.saturating_sub(1)) as f32
                    {
                        break;
                    }
                    let jitter = if rng.random::<bool>() {
                        branch_dir
                    } else {
                        0.0
                    };
                    let nbx =
                        (bx + jitter).clamp(1.0, (self.terminal_width.saturating_sub(2)) as f32);
                    let nby = by + 1.0;
                    segments.push((bx, by, nbx, nby, true));
                    bx = nbx;
                    by = nby;
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

    pub fn update(&mut self, tw: u16, th: u16, rng: &mut impl Rng) {
        self.terminal_width = tw;
        self.terminal_height = th;
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
                    self.next_strike_in = 30 + (rand::random::<u16>() % 200);
                }
            }
        }
    }

    pub fn render(&self, canvas: &mut HalfBlockCanvas, dark_bg: bool) {
        for bolt in &self.bolts {
            let life_ratio = bolt.age as f32 / bolt.max_age as f32;
            for &(x0, y0, x1, y1, is_branch) in &bolt.segments {
                let color = if self.flash_active {
                    if dark_bg {
                        Color::White
                    } else {
                        Color::Rgb(200, 200, 0)
                    }
                } else if self.afterglow_active {
                    if dark_bg {
                        Color::Rgb(180, 170, 220)
                    } else {
                        Color::Rgb(100, 90, 140)
                    }
                } else if is_branch {
                    if life_ratio > 0.5 {
                        if dark_bg {
                            Color::DarkGray
                        } else {
                            Color::Gray
                        }
                    } else if dark_bg {
                        Color::Rgb(120, 110, 180)
                    } else {
                        Color::Rgb(80, 70, 130)
                    }
                } else if life_ratio > 0.6 {
                    if dark_bg {
                        Color::Rgb(180, 160, 0)
                    } else {
                        Color::Rgb(140, 120, 0)
                    }
                } else if dark_bg {
                    Color::Yellow
                } else {
                    Color::Rgb(180, 160, 0)
                };
                canvas.draw_line(x0, y0, x1, y1, color);
            }
        }
        if self.flash_active {
            let y = (self.terminal_height.saturating_sub(2)) as f32;
            let w = self.terminal_width as f32;
            let color = if dark_bg {
                Color::White
            } else {
                Color::Rgb(200, 200, 0)
            };
            canvas.scatter_rect(0.0, y, w, 1.0, 0.3, color, 12345);
        }
    }
}
