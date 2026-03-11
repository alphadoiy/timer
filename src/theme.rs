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
            outline: Color::Rgb(46, 66, 92),
            accent: Color::Rgb(94, 211, 255),
            accent_soft: Color::Rgb(140, 179, 224),
            highlight: Color::Rgb(255, 196, 92),
            figlet_fg: Color::Rgb(244, 250, 255),
            figlet_bg: Color::Rgb(24, 36, 56),
            text: Color::Rgb(224, 236, 248),
            subtext: Color::Rgb(154, 173, 196),
            shadow: Color::Rgb(19, 30, 44),
            danger: Color::Rgb(255, 107, 107),
            success: Color::Rgb(92, 214, 156),
            hour_hand: Color::Rgb(236, 244, 255),
            minute_hand: Color::Rgb(120, 220, 255),
            second_hand: Color::Rgb(255, 104, 92),
        }
    }
}
