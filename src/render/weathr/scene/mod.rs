pub mod decorations;
pub mod ground;
pub mod house;

use crate::render::weathr::BrailleWeatherCanvas;
use ground::GroundWeather;

pub struct WorldScene {
    house: house::House,
    ground: ground::Ground,
    decorations: decorations::Decorations,
    width: u16,
    height: u16,
}

impl WorldScene {
    pub const GROUND_HEIGHT: u16 = 7;

    pub fn new(width: u16, height: u16) -> Self {
        Self {
            house: house::House,
            ground: ground::Ground,
            decorations: decorations::Decorations::new(),
            width,
            height,
        }
    }

    pub fn update_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    pub fn render_braille(
        &self,
        canvas: &mut BrailleWeatherCanvas,
        is_day: bool,
        weather: GroundWeather,
        dark_bg: bool,
    ) {
        let horizon_y = self.height.saturating_sub(Self::GROUND_HEIGHT);

        let house_width = self.house.width();
        let house_height = self.house.height();
        let house_x = (self.width / 2).saturating_sub(house_width / 2);
        let house_y = horizon_y.saturating_sub(house_height);

        self.ground.render_braille(
            canvas,
            self.width,
            Self::GROUND_HEIGHT,
            horizon_y,
            is_day,
            weather,
            dark_bg,
        );

        self.house.render_braille(canvas, house_x, house_y, is_day, dark_bg);

        self.decorations.render_braille(
            canvas,
            &decorations::DecorationRenderConfig {
                horizon_y,
                house_x,
                house_width,
                width: self.width,
                is_day,
            },
            dark_bg,
        );
    }
}
