#![allow(dead_code)]

pub mod airplanes;
pub mod birds;
pub mod butterflies;
pub mod chimney;
pub mod clouds;
pub mod fireflies;
pub mod fog;

pub mod moon;
pub mod raindrops;
pub mod snow;
pub mod stars;
pub mod sunny;
pub mod thunderstorm;

use crossterm::style::Color;

pub trait Animation {
    fn get_frame(&self, frame_number: usize) -> &[String];
    fn frame_count(&self) -> usize;

    fn get_color(&self) -> Color {
        Color::Reset
    }
}

pub struct AnimationController {
    current_frame: usize,
}

impl AnimationController {
    pub fn new() -> Self {
        Self { current_frame: 0 }
    }

    pub fn next_frame<A: Animation>(&mut self, animation: &A) -> usize {
        self.current_frame = (self.current_frame + 1) % animation.frame_count();
        self.current_frame
    }

    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.current_frame = 0;
    }
}

impl Default for AnimationController {
    fn default() -> Self {
        Self::new()
    }
}
