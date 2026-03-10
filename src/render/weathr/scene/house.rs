use crate::render::weathr::BrailleWeatherCanvas;
use ratatui::style::Color;

#[derive(Default)]
pub struct House;

impl House {
    pub const WIDTH: u16 = 24;
    pub const HEIGHT: u16 = 10;
    pub const CHIMNEY_X_OFFSET: u16 = 4;

    pub fn height(&self) -> u16 {
        Self::HEIGHT
    }

    pub fn width(&self) -> u16 {
        Self::WIDTH
    }

    pub fn render_braille(
        &self,
        canvas: &mut BrailleWeatherCanvas,
        x: u16,
        y: u16,
        is_day: bool,
        dark_bg: bool,
    ) {
        let xf = x as f32;
        let yf = y as f32;
        let w = Self::WIDTH as f32;
        let h = Self::HEIGHT as f32;

        let (wall_color, roof_color, window_color, door_color, chimney_color) = if is_day {
            if dark_bg {
                (
                    Color::Rgb(210, 180, 140),
                    Color::Rgb(160, 50, 50),
                    Color::Rgb(100, 200, 255),
                    Color::Rgb(139, 69, 19),
                    Color::Rgb(120, 100, 80),
                )
            } else {
                (
                    Color::Rgb(130, 100, 60),
                    Color::Rgb(120, 30, 30),
                    Color::Rgb(0, 100, 160),
                    Color::Rgb(80, 40, 10),
                    Color::Rgb(80, 60, 40),
                )
            }
        } else if dark_bg {
            (
                Color::Rgb(100, 70, 50),
                Color::Rgb(100, 30, 60),
                Color::Rgb(200, 180, 60),
                Color::Rgb(80, 40, 20),
                Color::Rgb(70, 55, 40),
            )
        } else {
            (
                Color::Rgb(70, 50, 30),
                Color::Rgb(70, 20, 40),
                Color::Rgb(140, 120, 0),
                Color::Rgb(50, 25, 10),
                Color::Rgb(50, 40, 30),
            )
        };

        let roof_peak_x = xf + w / 2.0;
        let roof_peak_y = yf;
        let roof_left_x = xf + 1.0;
        let roof_left_y = yf + h * 0.45;
        let roof_right_x = xf + w - 1.0;
        let roof_right_y = yf + h * 0.45;
        canvas.fill_triangle(
            roof_peak_x, roof_peak_y,
            roof_left_x, roof_left_y,
            roof_right_x, roof_right_y,
            roof_color,
        );

        let chimney_x = xf + Self::CHIMNEY_X_OFFSET as f32;
        let chimney_w = 2.0;
        let chimney_h = 2.5;
        canvas.fill_rect(chimney_x, roof_peak_y - 0.5, chimney_w, chimney_h, chimney_color);

        let wall_top = yf + h * 0.4;
        let wall_h = h * 0.6;
        let wall_w = w - 4.0;
        let wall_x = xf + 2.0;
        canvas.fill_rect(wall_x, wall_top, wall_w, wall_h, wall_color);

        let bottom_y = yf + h - 1.0;
        canvas.fill_rect(xf, bottom_y, w, 1.0, wall_color);

        let win_w = 2.0;
        let win_h = 1.5;
        let win_y = wall_top + 1.5;
        let win_positions = [wall_x + 2.0, wall_x + 6.0, wall_x + wall_w - 8.0, wall_x + wall_w - 4.0];
        for &wx in &win_positions {
            canvas.fill_rect(wx, win_y, win_w, win_h, window_color);
        }

        let door_w = 2.5;
        let door_h = 3.0;
        let door_x = wall_x + wall_w / 2.0 - door_w / 2.0;
        let door_y = yf + h - door_h;
        canvas.fill_rect(door_x, door_y, door_w, door_h, door_color);
    }
}
