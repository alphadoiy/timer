use crate::render::weathr::BrailleWeatherCanvas;
use ratatui::style::Color;

/// Centre-x and canopy-top-y of a deciduous tree (the only type that sheds leaves).
#[derive(Clone, Copy)]
pub struct TreeSpawnZone {
    pub x: f32,
    pub canopy_top_y: f32,
    pub canopy_rx: f32,
}

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

    /// Returns the spawn zones of the deciduous trees drawn this frame.
    pub fn render_braille(
        &self,
        canvas: &mut BrailleWeatherCanvas,
        config: &DecorationRenderConfig,
        dark_bg: bool,
    ) -> Vec<TreeSpawnZone> {
        let mut zones = Vec::new();

        let tree_x = config.house_x.saturating_sub(16) as f32;
        if tree_x > 2.0 {
            let zone = self.render_deciduous_tree(
                canvas,
                tree_x,
                config.horizon_y as f32,
                config.is_day,
                dark_bg,
            );
            zones.push(zone);
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
            if (pine_x as u16 + 8) < config.width {
                self.render_pine_tree(
                    canvas,
                    pine_x,
                    config.horizon_y as f32,
                    config.is_day,
                    dark_bg,
                );
            }
        }

        zones
    }

    /// Draws a deciduous tree and returns its canopy spawn zone for leaves.
    fn render_deciduous_tree(
        &self,
        canvas: &mut BrailleWeatherCanvas,
        x: f32,
        horizon: f32,
        is_day: bool,
        dark_bg: bool,
    ) -> TreeSpawnZone {
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
        let canopy_highlight = if is_day {
            if dark_bg { Color::Rgb(20, 170, 50) } else { Color::Rgb(10, 100, 25) }
        } else if dark_bg {
            Color::Rgb(10, 80, 20)
        } else {
            Color::Rgb(5, 50, 10)
        };

        let centre = x + 3.0;

        // Trunk: taller, with a slight fork at the top
        canvas.fill_rect(centre - 0.5, horizon - 6.0, 1.0, 6.0, trunk_color);
        // Small branches
        canvas.draw_line(centre, horizon - 5.5, centre - 1.5, horizon - 7.0, trunk_color);
        canvas.draw_line(centre, horizon - 5.0, centre + 1.5, horizon - 6.5, trunk_color);

        // Main canopy: layered ellipses for a large, rounded crown
        // Use fill_ellipse to get aspect-correct shapes
        canvas.fill_ellipse(centre, horizon - 9.0, 4.0, 2.0, canopy_color);
        canvas.fill_ellipse(centre - 1.5, horizon - 8.0, 3.0, 1.5, canopy_color);
        canvas.fill_ellipse(centre + 1.5, horizon - 8.0, 3.0, 1.5, canopy_color);
        canvas.fill_ellipse(centre, horizon - 7.5, 4.5, 1.8, canopy_color);

        // Highlight on upper-left for sun-lit effect
        canvas.fill_ellipse(centre - 1.0, horizon - 9.5, 2.0, 0.8, canopy_highlight);

        TreeSpawnZone {
            x: centre,
            canopy_top_y: horizon - 10.0,
            canopy_rx: 4.5,
        }
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
        let leaf_highlight = if is_day {
            if dark_bg { Color::Rgb(0, 150, 30) } else { Color::Rgb(0, 90, 15) }
        } else if dark_bg {
            Color::Rgb(0, 65, 15)
        } else {
            Color::Rgb(0, 40, 8)
        };

        let centre = x + 3.0;

        // Trunk
        canvas.fill_rect(centre - 0.5, horizon - 3.0, 1.0, 3.0, trunk_color);

        // Three stacked triangle tiers (bottom widest, top narrowest)
        // Bottom tier
        canvas.fill_triangle(
            centre, horizon - 7.0,
            x - 1.0, horizon - 3.0,
            x + 7.0, horizon - 3.0,
            leaf_color,
        );
        // Middle tier
        canvas.fill_triangle(
            centre, horizon - 10.0,
            x, horizon - 5.5,
            x + 6.0, horizon - 5.5,
            leaf_color,
        );
        // Top tier
        canvas.fill_triangle(
            centre, horizon - 13.0,
            x + 0.5, horizon - 8.5,
            x + 5.5, horizon - 8.5,
            leaf_highlight,
        );
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
