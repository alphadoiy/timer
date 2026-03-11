use crate::render::weathr::BrailleWeatherCanvas;
use ratatui::style::Color;
use std::f32::consts::TAU;

use super::Animation;

pub struct SunnyAnimation {
    frames: Vec<Vec<String>>,
}

impl SunnyAnimation {
    pub fn new() -> Self {
        Self {
            frames: vec![vec!["frame0".into()], vec!["frame1".into()]],
        }
    }

    pub fn render_braille(
        &self,
        canvas: &mut BrailleWeatherCanvas,
        frame_number: usize,
        dark_bg: bool,
    ) {
        let w = canvas.cell_width() as f32;
        let h = canvas.cell_height() as f32;
        let cx = w * 0.5;
        let cy = h * 0.25;
        self.render_braille_at(canvas, frame_number, dark_bg, cx, cy);
    }

    pub fn render_braille_at(
        &self,
        canvas: &mut BrailleWeatherCanvas,
        frame_number: usize,
        dark_bg: bool,
        cx: f32,
        cy: f32,
    ) {
        let core_r = 2.5;
        let ray_len = 2.4;
        let vertical_scale = 0.5;

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

        canvas.fill_circle(cx, cy, core_r, core_color);
        canvas.fill_circle(cx, cy, core_r + 0.3, glow_color);
        canvas.fill_circle(cx, cy, core_r - 0.5, core_color);

        let ray_count = 8;
        let phase = if frame_number.is_multiple_of(2) {
            0.0
        } else {
            TAU / (ray_count as f32 * 2.0)
        };
        for i in 0..ray_count {
            let theta = phase + (i as f32 / ray_count as f32) * TAU;
            let inner_r = core_r + 0.35;
            let outer_r = core_r + ray_len;
            let x0 = cx + theta.cos() * inner_r;
            let y0 = cy + theta.sin() * inner_r * vertical_scale;
            let x1 = cx + theta.cos() * outer_r;
            let y1 = cy + theta.sin() * outer_r * vertical_scale;
            canvas.draw_line(x0, y0, x1, y1, ray_color);
        }
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
