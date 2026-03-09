use crate::render::weathr::TerminalRenderer;
use crossterm::style::Color;
use std::io;

#[derive(Default)]
pub struct Ground;

#[derive(Clone, Copy, Default)]
pub struct GroundWeather {
    pub is_raining: bool,
    pub is_snowing: bool,
    pub is_thunderstorm: bool,
}

impl Ground {
    pub fn render_with_weather(
        &self,
        renderer: &mut TerminalRenderer,
        width: u16,
        height: u16,
        y_start: u16,
        is_day: bool,
        weather: GroundWeather,
    ) -> io::Result<()> {
        let width = width as usize;
        let height = height as usize;

        let (grass_colors, flower_colors, soil_color) = if weather.is_snowing {
            snow_palette(is_day)
        } else if weather.is_raining || weather.is_thunderstorm {
            wet_palette(is_day)
        } else {
            dry_palette(is_day)
        };

        for y in 0..height {
            for x in 0..width {
                let (ch, color) = if y == 0 {
                    grass_cell(x, y, &grass_colors, &flower_colors, &weather)
                } else {
                    soil_cell(x, y, soil_color, &weather)
                };
                renderer.render_char(x as u16, y_start + y as u16, ch, color)?;
            }
        }
        Ok(())
    }
}

fn pseudo_rand(x: usize, y: usize) -> u32 {
    ((x as u32 ^ 0x5DEECE6).wrapping_mul(y as u32 ^ 0xB)) % 100
}

fn dry_palette(is_day: bool) -> ([Color; 2], Vec<Color>, Color) {
    let grass = if is_day {
        [Color::Green, Color::DarkGreen]
    } else {
        [Color::DarkGreen, Color::Rgb { r: 0, g: 50, b: 0 }]
    };
    let flowers = if is_day {
        vec![Color::Magenta, Color::Red, Color::Cyan, Color::Yellow]
    } else {
        vec![Color::DarkMagenta, Color::DarkRed, Color::Blue, Color::DarkYellow]
    };
    let soil = if is_day {
        Color::Rgb { r: 101, g: 67, b: 33 }
    } else {
        Color::Rgb { r: 60, g: 40, b: 20 }
    };
    (grass, flowers, soil)
}

fn wet_palette(is_day: bool) -> ([Color; 2], Vec<Color>, Color) {
    let grass = if is_day {
        [Color::Rgb { r: 0, g: 100, b: 20 }, Color::Rgb { r: 0, g: 80, b: 15 }]
    } else {
        [Color::Rgb { r: 0, g: 55, b: 10 }, Color::Rgb { r: 0, g: 35, b: 5 }]
    };
    let flowers = if is_day {
        vec![Color::DarkMagenta, Color::DarkRed, Color::DarkCyan, Color::DarkYellow]
    } else {
        vec![Color::DarkMagenta, Color::DarkRed, Color::DarkBlue, Color::DarkYellow]
    };
    let soil = if is_day {
        Color::Rgb { r: 65, g: 42, b: 22 }
    } else {
        Color::Rgb { r: 38, g: 25, b: 12 }
    };
    (grass, flowers, soil)
}

fn snow_palette(is_day: bool) -> ([Color; 2], Vec<Color>, Color) {
    let grass = if is_day {
        [Color::White, Color::Grey]
    } else {
        [Color::Grey, Color::DarkGrey]
    };
    let flowers: Vec<Color> = vec![];
    let soil = if is_day {
        Color::Rgb { r: 200, g: 200, b: 210 }
    } else {
        Color::Rgb { r: 100, g: 100, b: 110 }
    };
    (grass, flowers, soil)
}

fn grass_cell(
    x: usize,
    y: usize,
    grass: &[Color; 2],
    flowers: &[Color],
    weather: &GroundWeather,
) -> (char, Color) {
    let r = pseudo_rand(x, y);
    if weather.is_snowing {
        if r < 30 { ('▓', grass[0]) } else { ('░', grass[1]) }
    } else if weather.is_raining || weather.is_thunderstorm {
        if r < 3 && !flowers.is_empty() {
            let f_idx = (x + y) % flowers.len();
            ('*', flowers[f_idx])
        } else if r < 12 {
            ('≈', Color::Rgb { r: 80, g: 120, b: 160 })
        } else if r < 20 {
            (',', grass[1])
        } else {
            ('^', grass[0])
        }
    } else if r < 5 && !flowers.is_empty() {
        let f_idx = (x + y) % flowers.len();
        ('*', flowers[f_idx])
    } else if r < 15 {
        (',', grass[1])
    } else {
        ('^', grass[0])
    }
}

fn soil_cell(x: usize, y: usize, soil_color: Color, weather: &GroundWeather) -> (char, Color) {
    let r = pseudo_rand(x, y);
    if weather.is_snowing {
        if r < 40 { ('░', Color::White) } else { (' ', soil_color) }
    } else if weather.is_raining || weather.is_thunderstorm {
        if r < 15 { ('≈', soil_color) } else if r < 22 { ('~', soil_color) } else { (' ', soil_color) }
    } else if r < 20 {
        ('~', soil_color)
    } else if r < 25 {
        ('.', soil_color)
    } else {
        (' ', soil_color)
    }
}
