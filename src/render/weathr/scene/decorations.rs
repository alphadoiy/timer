use crate::render::weathr::BrailleWeatherCanvas;
use ratatui::style::Color;

#[derive(Default)]
pub struct Decorations;

pub struct DecorationRenderConfig {
    pub horizon_y: u16,
    pub house_x: u16,
    pub house_width: u16,
    pub width: u16,
    pub is_day: bool,
}

impl Decorations {
    pub fn new() -> Self {
        Self
    }

    pub fn render_braille(
        &self,
        canvas: &mut BrailleWeatherCanvas,
        config: &DecorationRenderConfig,
        dark_bg: bool,
    ) {
        let tree_x = config.house_x.saturating_sub(12) as f32;
        if tree_x > 2.0 {
            self.render_deciduous_tree(
                canvas,
                tree_x,
                config.horizon_y as f32,
                config.is_day,
                dark_bg,
            );
        }

        let fence_x = (config.house_x + config.house_width + 2) as f32;
        if (fence_x as u16) < config.width.saturating_sub(5) {
            self.render_fence(canvas, fence_x, config.horizon_y as f32, config.is_day, dark_bg);
        }

        let mailbox_x = tree_x - 6.0;
        if mailbox_x > 1.0 {
            self.render_mailbox(
                canvas,
                mailbox_x,
                config.horizon_y as f32,
                config.is_day,
                dark_bg,
            );
        }

        if config.width > 60 {
            let pine_x = (config.house_x + config.house_width + 14) as f32;
            if (pine_x as u16 + 6) < config.width {
                self.render_pine_tree(
                    canvas,
                    pine_x,
                    config.horizon_y as f32,
                    config.is_day,
                    dark_bg,
                );
            }
        }
    }

    fn render_deciduous_tree(
        &self,
        canvas: &mut BrailleWeatherCanvas,
        x: f32,
        horizon: f32,
        is_day: bool,
        dark_bg: bool,
    ) {
        let trunk_color = if dark_bg {
            Color::Rgb(100, 70, 40)
        } else {
            Color::Rgb(60, 40, 20)
        };
        let canopy_color = if is_day {
            if dark_bg { Color::Rgb(0, 140, 30) } else { Color::Rgb(0, 80, 15) }
        } else if dark_bg {
            Color::Rgb(0, 60, 10)
        } else {
            Color::Rgb(0, 35, 5)
        };

        canvas.fill_rect(x + 1.5, horizon - 3.0, 1.0, 3.0, trunk_color);
        canvas.fill_circle(x + 2.0, horizon - 5.0, 2.5, canopy_color);
        canvas.fill_circle(x + 1.0, horizon - 4.5, 1.8, canopy_color);
        canvas.fill_circle(x + 3.0, horizon - 4.5, 1.8, canopy_color);
    }

    fn render_pine_tree(
        &self,
        canvas: &mut BrailleWeatherCanvas,
        x: f32,
        horizon: f32,
        is_day: bool,
        dark_bg: bool,
    ) {
        let trunk_color = if dark_bg {
            Color::Rgb(100, 70, 40)
        } else {
            Color::Rgb(60, 40, 20)
        };
        let leaf_color = if is_day {
            if dark_bg { Color::Rgb(0, 120, 20) } else { Color::Rgb(0, 70, 10) }
        } else if dark_bg {
            Color::Rgb(0, 50, 10)
        } else {
            Color::Rgb(0, 30, 5)
        };

        canvas.fill_rect(x + 2.0, horizon - 2.0, 1.0, 2.0, trunk_color);
        canvas.fill_triangle(x + 2.5, horizon - 8.0, x, horizon - 2.0, x + 5.0, horizon - 2.0, leaf_color);
        canvas.fill_triangle(x + 2.5, horizon - 6.5, x + 0.5, horizon - 3.0, x + 4.5, horizon - 3.0, leaf_color);
    }

    fn render_fence(
        &self,
        canvas: &mut BrailleWeatherCanvas,
        x: f32,
        horizon: f32,
        is_day: bool,
        dark_bg: bool,
    ) {
        let color = if is_day {
            if dark_bg { Color::White } else { Color::Rgb(100, 100, 100) }
        } else if dark_bg {
            Color::Gray
        } else {
            Color::Rgb(70, 70, 70)
        };
        let post_count = 5;
        let spacing = 2.0;
        let fence_h = 2.0;
        for i in 0..post_count {
            let px = x + i as f32 * spacing;
            canvas.draw_line(px, horizon - fence_h, px, horizon, color);
        }
        let rail_y1 = horizon - fence_h * 0.8;
        let rail_y2 = horizon - fence_h * 0.3;
        let end_x = x + (post_count - 1) as f32 * spacing;
        canvas.draw_line(x, rail_y1, end_x, rail_y1, color);
        canvas.draw_line(x, rail_y2, end_x, rail_y2, color);
    }

    fn render_mailbox(
        &self,
        canvas: &mut BrailleWeatherCanvas,
        x: f32,
        horizon: f32,
        is_day: bool,
        dark_bg: bool,
    ) {
        let color = if is_day {
            if dark_bg { Color::Blue } else { Color::Rgb(0, 0, 140) }
        } else if dark_bg {
            Color::Rgb(40, 40, 120)
        } else {
            Color::Rgb(20, 20, 80)
        };
        canvas.draw_line(x + 1.0, horizon - 3.0, x + 1.0, horizon, color);
        canvas.fill_rect(x, horizon - 4.0, 2.5, 1.5, color);
    }
}
