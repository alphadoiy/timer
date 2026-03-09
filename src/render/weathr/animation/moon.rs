use crate::render::weathr::TerminalRenderer;
use crossterm::style::Color;
use std::io;

const BODY_FG: Color = Color::Rgb { r: 230, g: 215, b: 140 };
const BODY_FG_DIM: Color = Color::Rgb { r: 200, g: 185, b: 115 };
const CRATER_FG: Color = Color::Rgb { r: 170, g: 155, b: 95 };
const EDGE_FG: Color = Color::Rgb { r: 240, g: 230, b: 170 };

pub struct MoonSystem {
    phase: f64,
    x: u16,
    y: u16,
}

impl MoonSystem {
    pub fn new(terminal_width: u16, terminal_height: u16, phase: Option<f64>) -> Self {
        Self {
            phase: phase.unwrap_or(0.5),
            x: (terminal_width / 4) + 10,
            y: (terminal_height / 4) + 2,
        }
    }

    pub fn set_phase(&mut self, phase: f64) {
        self.phase = phase;
    }

    pub fn update(&mut self, terminal_width: u16, terminal_height: u16) {
        self.x = (terminal_width / 4 * 3).min(terminal_width.saturating_sub(15));
        self.y = (terminal_height / 4).max(2);
    }

    pub fn render(&self, renderer: &mut TerminalRenderer) -> io::Result<()> {
        let step = (self.phase * 8.0).round() as usize % 8;

        let art = match step {
            0 => vec![
                "                 ",
                "                 ",
                "                 ",
                "                 ",
                "                 ",
                "                 ",
            ],
            1 => vec![
                "             .    ",
                "            . `.  ",
                "              B:  ",
                "              B:  ",
                "            . .'  ",
                "             `    ",
            ],
            2 => vec![
                "            _     ",
                "           |B `.  ",
                "           |BBB: ",
                "           |BBB: ",
                "           |B .'  ",
                "           |-'    ",
            ],
            3 => vec![
                "         ..._     ",
                "       .'BBBB`.   ",
                "      |BBBBoBBB:  ",
                "      |B.BBBBoB:  ",
                "       `.BBBBB'   ",
                "         `...-'   ",
            ],
            4 => vec![
                "       _..._      ",
                "     .'BoBBBB`.    ",
                "    :BBBBBoBBBB:   ",
                "    :BBoBBBBB.B:   ",
                "    `.BBBBBoBB.'   ",
                "      `-...-'     ",
            ],
            5 => vec![
                "       _...       ",
                "     .'BBBB`.     ",
                "    :BBBoBBBB|    ",
                "    :BoBBBB.B|    ",
                "    `.BBBBB.'     ",
                "      `-...-'     ",
            ],
            6 => vec![
                "        _         ",
                "      .' B|       ",
                "     :BBBB|       ",
                "     :BBBB|       ",
                "      `.B |       ",
                "        `-|       ",
            ],
            7 => vec![
                "        .         ",
                "      .' .        ",
                "     :            ",
                "     :            ",
                "      '. .        ",
                "        `         ",
            ],
            _ => vec![],
        };

        for (i, line) in art.iter().enumerate() {
            let y = self.y + i as u16;
            for (j, ch) in line.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }

                let x = self.x + j as u16;

                if ch == 'B' {
                    renderer.render_char(x, y, '▓', BODY_FG)?;
                } else if ch == 'o' {
                    renderer.render_char(x, y, '○', CRATER_FG)?;
                } else if ch == '.' && self.is_interior_dot(line, j) {
                    renderer.render_char(x, y, '░', BODY_FG_DIM)?;
                } else {
                    renderer.render_char(x, y, ch, EDGE_FG)?;
                }
            }
        }
        Ok(())
    }

    fn is_interior_dot(&self, line: &str, pos: usize) -> bool {
        let chars: Vec<char> = line.chars().collect();
        let has_body_left = chars[..pos].iter().any(|&c| c == 'B' || c == 'o');
        let has_body_right = chars[pos + 1..].iter().any(|&c| c == 'B' || c == 'o');
        has_body_left && has_body_right
    }
}
