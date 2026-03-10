use crate::render::weathr::BrailleWeatherCanvas;
use ratatui::style::Color;

#[derive(Default)]
pub struct Ground;

#[derive(Clone, Copy, Default)]
pub struct GroundWeather {
    pub is_raining: bool,
    pub is_snowing: bool,
    pub is_thunderstorm: bool,
}

impl Ground {
    #[allow(clippy::too_many_arguments)]
    pub fn render_braille(
        &self,
        canvas: &mut BrailleWeatherCanvas,
        width: u16,
        height: u16,
        y_start: u16,
        is_day: bool,
        weather: GroundWeather,
        dark_bg: bool,
    ) {
        let (grass_color, grass_alt, flower_colors, soil_color) = if weather.is_snowing {
            snow_palette(is_day, dark_bg)
        } else if weather.is_raining || weather.is_thunderstorm {
            wet_palette(is_day, dark_bg)
        } else {
            dry_palette(is_day, dark_bg)
        };

        let w = width as f32;
        canvas.scatter_rect(
            0.0,
            y_start as f32,
            w,
            1.0,
            0.6,
            grass_color,
            42,
        );
        canvas.scatter_rect(
            0.0,
            y_start as f32,
            w,
            1.0,
            0.2,
            grass_alt,
            99,
        );

        if !flower_colors.is_empty() {
            for (i, &fc) in flower_colors.iter().enumerate() {
                canvas.scatter_rect(
                    0.0,
                    y_start as f32,
                    w,
                    1.0,
                    0.03,
                    fc,
                    200 + i as u32,
                );
            }
        }

        if weather.is_raining || weather.is_thunderstorm {
            let puddle_color = if dark_bg {
                Color::Rgb(80, 120, 160)
            } else {
                Color::Rgb(40, 60, 100)
            };
            canvas.scatter_rect(
                0.0,
                y_start as f32,
                w,
                1.0,
                0.08,
                puddle_color,
                777,
            );
        }

        for row in 1..height {
            let y = y_start + row;
            let density = 0.15 - (row as f32 / height as f32) * 0.1;
            canvas.scatter_rect(
                0.0,
                y as f32,
                w,
                1.0,
                density.max(0.02),
                soil_color,
                row as u32 * 31,
            );
        }
    }
}

fn dry_palette(is_day: bool, dark_bg: bool) -> (Color, Color, Vec<Color>, Color) {
    if is_day {
        if dark_bg {
            (
                Color::Green,
                Color::Rgb(0, 160, 0),
                vec![Color::Magenta, Color::Red, Color::Cyan, Color::Yellow],
                Color::Rgb(101, 67, 33),
            )
        } else {
            (
                Color::Rgb(0, 100, 0),
                Color::Rgb(0, 80, 0),
                vec![
                    Color::Rgb(140, 0, 100),
                    Color::Rgb(160, 0, 0),
                    Color::Rgb(0, 100, 120),
                    Color::Rgb(160, 140, 0),
                ],
                Color::Rgb(70, 45, 20),
            )
        }
    } else if dark_bg {
        (
            Color::Rgb(0, 100, 0),
            Color::Rgb(0, 50, 0),
            vec![Color::Rgb(100, 0, 80), Color::Rgb(120, 0, 0)],
            Color::Rgb(60, 40, 20),
        )
    } else {
        (
            Color::Rgb(0, 60, 0),
            Color::Rgb(0, 35, 0),
            vec![Color::Rgb(80, 0, 60), Color::Rgb(90, 0, 0)],
            Color::Rgb(40, 25, 10),
        )
    }
}

fn wet_palette(is_day: bool, dark_bg: bool) -> (Color, Color, Vec<Color>, Color) {
    if is_day {
        if dark_bg {
            (
                Color::Rgb(0, 100, 20),
                Color::Rgb(0, 80, 15),
                vec![Color::Rgb(120, 0, 80), Color::Rgb(140, 0, 0)],
                Color::Rgb(65, 42, 22),
            )
        } else {
            (
                Color::Rgb(0, 70, 15),
                Color::Rgb(0, 50, 10),
                vec![Color::Rgb(80, 0, 60), Color::Rgb(100, 0, 0)],
                Color::Rgb(45, 28, 14),
            )
        }
    } else if dark_bg {
        (
            Color::Rgb(0, 55, 10),
            Color::Rgb(0, 35, 5),
            vec![],
            Color::Rgb(38, 25, 12),
        )
    } else {
        (
            Color::Rgb(0, 40, 8),
            Color::Rgb(0, 25, 4),
            vec![],
            Color::Rgb(28, 18, 8),
        )
    }
}

fn snow_palette(is_day: bool, dark_bg: bool) -> (Color, Color, Vec<Color>, Color) {
    if is_day {
        if dark_bg {
            (Color::White, Color::Gray, vec![], Color::Rgb(200, 200, 210))
        } else {
            (
                Color::Rgb(120, 120, 130),
                Color::Rgb(100, 100, 110),
                vec![],
                Color::Rgb(100, 100, 110),
            )
        }
    } else if dark_bg {
        (Color::Gray, Color::DarkGray, vec![], Color::Rgb(100, 100, 110))
    } else {
        (
            Color::Rgb(80, 80, 90),
            Color::Rgb(60, 60, 70),
            vec![],
            Color::Rgb(60, 60, 70),
        )
    }
}
