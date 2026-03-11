pub mod animation;
pub mod halfblock_canvas;
pub mod scene;
pub mod types;
pub(crate) mod weather_scene;

pub use halfblock_canvas::HalfBlockCanvas;

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn halfblock_canvas_basic() {
        let area = Rect::new(0, 0, 10, 5);
        let mut canvas = HalfBlockCanvas::new(area);
        canvas.plot(0, 0, ratatui::style::Color::White);
        assert_eq!(canvas.cell_width(), 10);
        assert_eq!(canvas.cell_height(), 5);
    }
}
