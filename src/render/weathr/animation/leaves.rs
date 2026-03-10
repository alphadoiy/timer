use crate::render::weathr::BrailleWeatherCanvas;
use rand::prelude::*;
use ratatui::style::Color;

const LEAF_COLORS_DARK: [(u8, u8, u8); 6] = [
    (255, 165, 0),
    (218, 165, 32),
    (184, 134, 11),
    (205, 92, 92),
    (160, 82, 45),
    (139, 69, 19),
];

const LEAF_COLORS_LIGHT: [(u8, u8, u8); 6] = [
    (180, 100, 0),
    (150, 110, 0),
    (120, 80, 0),
    (150, 50, 50),
    (110, 50, 20),
    (90, 40, 10),
];

struct Leaf {
    x: f32,
    y: f32,
    fall_speed: f32,
    sway_speed: f32,
    sway_phase: f32,
    sway_amplitude: f32,
    color_idx: usize,
}

impl Leaf {
    fn new(tw: u16, rng: &mut impl Rng) -> Self {
        let x = rng.random::<f32>() * tw as f32;
        // Always spawn just above the top so they drift in naturally.
        let y = -(rng.random::<f32>() * 4.0);
        Self {
            x,
            y,
            fall_speed: 0.12 + rng.random::<f32>() * 0.15,
            sway_speed: 0.04 + rng.random::<f32>() * 0.08,
            sway_phase: rng.random::<f32>() * std::f32::consts::TAU,
            sway_amplitude: 0.4 + rng.random::<f32>() * 1.2,
            color_idx: (rng.random::<u32>() % LEAF_COLORS_DARK.len() as u32) as usize,
        }
    }

    fn update(&mut self) {
        self.y += self.fall_speed;
        self.sway_phase += self.sway_speed;
        if self.sway_phase > std::f32::consts::TAU {
            self.sway_phase -= std::f32::consts::TAU;
        }
        self.x += self.sway_phase.sin() * self.sway_amplitude * 0.08;
    }

    fn is_offscreen(&self, th: u16) -> bool {
        self.y > th as f32
    }
}

pub struct FallingLeaves {
    leaves: Vec<Leaf>,
    spawn_counter: u32,
    spawn_rate: u32,
    terminal_width: u16,
    terminal_height: u16,
}

impl FallingLeaves {
    pub fn new(tw: u16, th: u16) -> Self {
        // Start empty — leaves drift in one at a time from the top.
        // This avoids the "shower of coloured blocks" on startup.
        let cap = (tw / 8).max(10) as usize;
        Self {
            leaves: Vec::with_capacity(cap),
            spawn_counter: 0,
            spawn_rate: 22,
            terminal_width: tw,
            terminal_height: th,
        }
    }

    pub fn update(&mut self, tw: u16, th: u16, rng: &mut impl Rng) {
        self.terminal_width = tw;
        self.terminal_height = th;
        for leaf in &mut self.leaves {
            leaf.update();
        }
        self.leaves.retain(|l| !l.is_offscreen(th));
        self.spawn_counter += 1;
        if self.spawn_counter >= self.spawn_rate {
            self.spawn_counter = 0;
            if rng.random::<f32>() < 0.6 {
                self.leaves.push(Leaf::new(tw, rng));
            }
        }
        let max = (tw / 8).max(10) as usize;
        if self.leaves.len() > max {
            self.leaves.truncate(max);
        }
    }

    pub fn render_braille(&self, canvas: &mut BrailleWeatherCanvas, dark_bg: bool) {
        let palette = if dark_bg { &LEAF_COLORS_DARK } else { &LEAF_COLORS_LIGHT };
        for leaf in &self.leaves {
            let (r, g, b) = palette[leaf.color_idx % palette.len()];
            let color = Color::Rgb(r, g, b);
            // Two-dot "leaf" shape
            canvas.plot_f(leaf.x, leaf.y, color);
            canvas.plot_f(leaf.x + 0.5, leaf.y + 0.25, color);
        }
    }
}
