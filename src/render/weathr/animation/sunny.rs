use crate::render::weathr::TerminalRenderer;
use crossterm::style::Color;
use std::io;

use super::Animation;

const CORE_FG: Color = Color::Rgb { r: 255, g: 220, b: 80 };
const CORE_FILL: Color = Color::Rgb { r: 255, g: 200, b: 40 };
const RAY_FG: Color = Color::Rgb { r: 255, g: 200, b: 50 };
const RAY_TIP_FG: Color = Color::Rgb { r: 255, g: 180, b: 60 };

pub struct SunnyAnimation {
    frames: Vec<Vec<String>>,
}

impl SunnyAnimation {
    pub fn new() -> Self {
        let frames = vec![Self::create_frame_1(), Self::create_frame_2()];
        Self { frames }
    }

    fn create_frame_1() -> Vec<String> {
        vec![
            "      ;   :   ;".to_string(),
            "   .   \\_,!,_/   ,".to_string(),
            "    `.,'CCCCC`.,'".to_string(),
            "     /CCCCCCCCC\\".to_string(),
            "~ -- :CCCCCCCCC: -- ~".to_string(),
            "     \\CCCCCCCCC/".to_string(),
            "    ,'`._CCC_.'`.".to_string(),
            "   '   / `!` \\   `".to_string(),
            "      ;   :   ;".to_string(),
        ]
    }

    fn create_frame_2() -> Vec<String> {
        vec![
            "      .   |   .".to_string(),
            "   ;   \\_,|,_/   ;".to_string(),
            "    `.,'CCCCC`.,'".to_string(),
            "     /CCCCCCCCC\\".to_string(),
            "~ -- |CCCCCCCCC| -- ~".to_string(),
            "     \\CCCCCCCCC/".to_string(),
            "    ,'`._CCC_.'`.".to_string(),
            "   ;   / `|` \\   ;".to_string(),
            "      .   |   .".to_string(),
        ]
    }

    pub fn render_colored(
        &self,
        renderer: &mut TerminalRenderer,
        frame_number: usize,
        y_offset: u16,
    ) -> io::Result<()> {
        let frame = &self.frames[frame_number % self.frames.len()];
        let max_width = frame.iter().map(|l| l.len()).max().unwrap_or(0);
        let (w, _) = renderer.size();
        let start_col = if (w as usize) > max_width {
            (w as usize - max_width) / 2
        } else {
            0
        };

        for (idx, line) in frame.iter().enumerate() {
            let row = y_offset + idx as u16;
            for (char_idx, ch) in line.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }
                let col = start_col as u16 + char_idx as u16;

                if ch == 'C' {
                    renderer.render_char(col, row, '█', CORE_FILL)?;
                } else {
                    let color = classify_sun_char(ch, idx, frame.len());
                    renderer.render_char(col, row, ch, color)?;
                }
            }
        }
        Ok(())
    }
}

fn classify_sun_char(ch: char, row: usize, total_rows: usize) -> Color {
    let center = total_rows / 2;
    let dist = (row as i16 - center as i16).unsigned_abs() as usize;
    match ch {
        '!' | ',' | '.' | ';' | ':' | '|' => {
            if dist >= 3 { RAY_TIP_FG } else { RAY_FG }
        }
        '~' | '-' => RAY_FG,
        '/' | '\\' | '`' | '\'' | '_' => CORE_FG,
        _ => RAY_FG,
    }
}

impl Animation for SunnyAnimation {
    fn get_frame(&self, frame_number: usize) -> &[String] {
        &self.frames[frame_number % self.frames.len()]
    }

    fn frame_count(&self) -> usize {
        self.frames.len()
    }

    fn get_color(&self) -> Color {
        Color::Yellow
    }
}

impl Default for SunnyAnimation {
    fn default() -> Self {
        Self::new()
    }
}
