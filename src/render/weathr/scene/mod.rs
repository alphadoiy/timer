pub mod decorations;
pub mod ground;
pub mod house;

use crate::render::weathr::TerminalRenderer;
use std::io;

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

    pub fn render(&self, renderer: &mut TerminalRenderer, is_day: bool) -> io::Result<()> {
        let horizon_y = self.height.saturating_sub(Self::GROUND_HEIGHT);

        let house_width = self.house.width();
        let house_height = self.house.height();
        let house_x = (self.width / 2).saturating_sub(house_width / 2);
        let house_y = horizon_y.saturating_sub(house_height);

        self.ground
            .render(renderer, self.width, Self::GROUND_HEIGHT, horizon_y, is_day)?;

        self.house.render(renderer, house_x, house_y, is_day)?;

        self.decorations.render(
            renderer,
            &decorations::DecorationRenderConfig {
                horizon_y,
                house_x,
                house_width,
                width: self.width,
                is_day,
            },
        )?;

        Ok(())
    }
}
