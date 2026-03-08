use ratatui::style::Color;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub outline: Color,
    pub accent: Color,
    pub accent_soft: Color,
    pub highlight: Color,
    pub figlet_fg: Color,
    pub figlet_bg: Color,
    pub text: Color,
    pub subtext: Color,
    pub shadow: Color,
    pub danger: Color,
    pub success: Color,
    pub hour_hand: Color,
    pub minute_hand: Color,
    pub second_hand: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            outline: Color::Rgb(0, 0, 0),
            accent: Color::Rgb(12, 18, 28),
            accent_soft: Color::Rgb(56, 71, 96),
            highlight: Color::Rgb(18, 30, 48),
            figlet_fg: Color::Rgb(255, 255, 255),
            figlet_bg: Color::Rgb(24, 36, 56),
            text: Color::Rgb(20, 27, 38),
            subtext: Color::Rgb(92, 102, 118),
            shadow: Color::Rgb(34, 45, 63),
            danger: Color::Rgb(220, 86, 48),
            success: Color::Rgb(52, 153, 104),
            hour_hand: Color::Rgb(24, 34, 50),
            minute_hand: Color::Rgb(12, 22, 36),
            second_hand: Color::Rgb(255, 85, 85),
        }
    }
}
