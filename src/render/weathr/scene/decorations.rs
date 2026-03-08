use crate::render::weathr::TerminalRenderer;
use crossterm::style::Color;
use std::io;

#[derive(Default)]
pub struct Decorations;

pub struct DecorationRenderConfig {
    pub horizon_y: u16,
    pub house_x: u16,
    pub house_width: u16,
    pub width: u16,
    pub is_day: bool,
}

impl Decorations {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        renderer: &mut TerminalRenderer,
        config: &DecorationRenderConfig,
    ) -> io::Result<()> {
        let (tree_lines, tree_color) = self.get_tree(config.is_day);
        let tree_height = tree_lines.len() as u16;
        let tree_y = config.horizon_y.saturating_sub(tree_height);
        let tree_x = config.house_x.saturating_sub(20);

        if tree_x > 0 {
            for (i, line) in tree_lines.iter().enumerate() {
                for (j, ch) in line.chars().enumerate() {
                    if ch != ' ' {
                        renderer.render_char(
                            tree_x + j as u16,
                            tree_y + i as u16,
                            ch,
                            tree_color,
                        )?;
                    }
                }
            }
        }

        let (fence_lines, fence_color) = self.get_fence(config.is_day);
        let fence_height = fence_lines.len() as u16;
        let fence_y = config.horizon_y.saturating_sub(fence_height);
        let fence_x = config.house_x + config.house_width + 2;

        if fence_x < config.width {
            for (i, line) in fence_lines.iter().enumerate() {
                for (j, ch) in line.chars().enumerate() {
                    if ch != ' ' {
                        renderer.render_char(
                            fence_x + j as u16,
                            fence_y + i as u16,
                            ch,
                            fence_color,
                        )?;
                    }
                }
            }
        }

        let (mailbox_lines, mailbox_color) = self.get_mailbox(config.is_day);
        let mailbox_height = mailbox_lines.len() as u16;
        let mailbox_x = tree_x.saturating_sub(10);
        let mailbox_y = config.horizon_y.saturating_sub(mailbox_height);

        if mailbox_x < config.width {
            for (i, line) in mailbox_lines.iter().enumerate() {
                for (j, ch) in line.chars().enumerate() {
                    if ch != ' ' {
                        renderer.render_char(
                            mailbox_x + j as u16,
                            mailbox_y + i as u16,
                            ch,
                            mailbox_color,
                        )?;
                    }
                }
            }
        }

        if config.width > 120 {
            let (pine_lines, pine_color) = self.get_pine_tree(config.is_day);
            let pine_height = pine_lines.len() as u16;
            let pine_x = config.house_x + config.house_width + 18;
            let pine_y = config.horizon_y.saturating_sub(pine_height);

            if pine_x + 10 < config.width {
                for (i, line) in pine_lines.iter().enumerate() {
                    for (j, ch) in line.chars().enumerate() {
                        if ch != ' ' {
                            renderer.render_char(
                                pine_x + j as u16,
                                pine_y + i as u16,
                                ch,
                                pine_color,
                            )?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn get_tree(&self, is_day: bool) -> (Vec<&'static str>, Color) {
        (
            vec![
                "      ####      ",
                "    ########    ",
                "   ##########   ",
                "    ########    ",
                "      _||_      ",
            ],
            if is_day {
                Color::DarkGreen
            } else {
                Color::Rgb { r: 0, g: 50, b: 0 }
            },
        )
    }

    fn get_fence(&self, is_day: bool) -> (Vec<&'static str>, Color) {
        (
            vec!["|--|--|--|--|", "|  |  |  |  |"],
            if is_day { Color::White } else { Color::Grey },
        )
    }

    fn get_mailbox(&self, is_day: bool) -> (Vec<&'static str>, Color) {
        (
            vec![" ___ ", "|___|", "  |  "],
            if is_day { Color::Blue } else { Color::DarkBlue },
        )
    }

    fn get_pine_tree(&self, is_day: bool) -> (Vec<&'static str>, Color) {
        (
            vec![
                "    *    ",
                "   ***   ",
                "  *****  ",
                " ******* ",
                "   |||   ",
            ],
            if is_day {
                Color::DarkGreen
            } else {
                Color::Rgb { r: 0, g: 50, b: 0 }
            },
        )
    }
}
