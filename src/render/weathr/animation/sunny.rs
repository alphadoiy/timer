use crate::render::weathr::HalfBlockCanvas;
use ratatui::style::Color;

use super::Animation;

const SUN_FRAME_EVEN: &[&str] = &[
    ".....R.....",
    "...R.G.R...",
    "..RRGGGRR..",
    "...GCCCG...",
    ".RGCCCCCRG.",
    "RGCCCCCCCRG",
    ".RGCCCCCRG.",
    "...GCCCG...",
    "..RRGGGRR..",
    "...R.G.R...",
    ".....R.....",
];

const SUN_FRAME_ODD: &[&str] = &[
    "..R.....R..",
    "...RGGGR...",
    ".R.GCCCG.R.",
    "..GCCCCCG..",
    ".GCCCCCCCG.",
    "..CCCCCCC..",
    ".GCCCCCCCG.",
    "..GCCCCCG..",
    ".R.GCCCG.R.",
    "...RGGGR...",
    "..R.....R..",
];

pub struct SunnyAnimation {
    frames: Vec<Vec<String>>,
}

impl SunnyAnimation {
    pub fn new() -> Self {
        Self {
            frames: vec![vec!["frame0".into()], vec!["frame1".into()]],
        }
    }

    pub fn render(&self, canvas: &mut HalfBlockCanvas, frame_number: usize, dark_bg: bool) {
        let w = canvas.cell_width() as f32;
        let h = canvas.cell_height() as f32;
        let cx = w * 0.5;
        let cy = h * 0.25;
        self.render_at(canvas, frame_number, dark_bg, cx, cy);
    }

    pub fn render_at(
        &self,
        canvas: &mut HalfBlockCanvas,
        frame_number: usize,
        dark_bg: bool,
        cx: f32,
        cy: f32,
    ) {
        let (core_color, ray_color, glow_color) = if dark_bg {
            (
                Color::Rgb(255, 220, 80),
                Color::Rgb(255, 200, 50),
                Color::Rgb(255, 180, 60),
            )
        } else {
            (
                Color::Rgb(200, 150, 0),
                Color::Rgb(180, 130, 0),
                Color::Rgb(160, 110, 0),
            )
        };

        canvas.dither_ellipse(cx, cy, 5.8, 3.8, 0.22, glow_color, frame_number as u32 + 11);
        canvas.dither_ellipse(
            cx - 0.5,
            cy - 0.4,
            2.0,
            0.9,
            0.4,
            glow_color,
            frame_number as u32 + 29,
        );
        let sprite = if frame_number.is_multiple_of(2) {
            SUN_FRAME_EVEN
        } else {
            SUN_FRAME_ODD
        };
        render_sun_sprite(canvas, cx, cy, sprite, core_color, ray_color, glow_color);
    }
}

impl Animation for SunnyAnimation {
    fn get_frame(&self, frame_number: usize) -> &[String] {
        &self.frames[frame_number % self.frames.len()]
    }
    fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

impl Default for SunnyAnimation {
    fn default() -> Self {
        Self::new()
    }
}

fn render_sun_sprite(
    canvas: &mut HalfBlockCanvas,
    cx: f32,
    cy: f32,
    sprite: &[&str],
    core_color: Color,
    ray_color: Color,
    glow_color: Color,
) {
    let base_x = cx.round() as i32 - (sprite[0].len() as i32 / 2);
    let base_y = (cy * 2.0).round() as i32 - (sprite.len() as i32 / 2);
    for (row, line) in sprite.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            let color = match ch {
                'C' => Some(core_color),
                'R' => Some(ray_color),
                'G' => Some(glow_color),
                _ => None,
            };
            if let Some(color) = color {
                canvas.plot(base_x + col as i32, base_y + row as i32, color);
            }
        }
    }
}
