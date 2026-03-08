use ratatui::style::Color;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub outline: Color,
    pub accent: Color,
    pub accent_soft: Color,
    pub highlight: Color,
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
            outline: Color::Rgb(170, 182, 204),
            accent: Color::Rgb(255, 255, 255),
            accent_soft: Color::Rgb(164, 178, 202),
            highlight: Color::Rgb(226, 236, 255),
            text: Color::Rgb(244, 248, 255),
            subtext: Color::Rgb(92, 102, 118),
            shadow: Color::Rgb(52, 58, 72),
            danger: Color::Rgb(255, 132, 58),
            success: Color::Rgb(109, 224, 159),
            hour_hand: Color::Rgb(46, 58, 78),
            minute_hand: Color::Rgb(24, 32, 44),
            second_hand: Color::Rgb(255, 85, 85),
        }
    }
}
