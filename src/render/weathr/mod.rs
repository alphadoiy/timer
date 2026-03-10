pub mod animation;
pub mod braille_canvas;
pub mod scene;
pub mod types;
pub(crate) mod weather_scene;

pub use braille_canvas::BrailleWeatherCanvas;

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn braille_canvas_basic() {
        let area = Rect::new(0, 0, 10, 5);
        let mut canvas = BrailleWeatherCanvas::new(area);
        canvas.plot(0, 0, ratatui::style::Color::White);
        assert_eq!(canvas.cell_width(), 10);
        assert_eq!(canvas.cell_height(), 5);
    }
}
