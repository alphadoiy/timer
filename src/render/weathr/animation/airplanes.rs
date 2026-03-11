use crate::render::weathr::HalfBlockCanvas;
use rand::prelude::*;
use ratatui::style::Color;

struct Airplane {
    x: f32,
    y: f32,
    speed: f32,
}

pub struct AirplaneSystem {
    planes: Vec<Airplane>,
    terminal_width: u16,
    terminal_height: u16,
    spawn_cooldown: u16,
}

impl AirplaneSystem {
    pub fn new(tw: u16, th: u16) -> Self {
        Self {
            planes: Vec::with_capacity(2),
            terminal_width: tw,
            terminal_height: th,
            spawn_cooldown: 0,
        }
    }

    pub fn update(&mut self, tw: u16, th: u16, rng: &mut impl Rng) {
        self.terminal_width = tw;
        self.terminal_height = th;
        for plane in &mut self.planes {
            plane.x += plane.speed;
        }
        // Remove planes that have fully cleared the right edge (fuselage+contrail ~13 cells)
        self.planes.retain(|p| p.x < tw as f32 + 13.0);
        self.spawn_cooldown = self.spawn_cooldown.saturating_sub(1);
        if self.spawn_cooldown == 0 && rng.random::<f32>() < 0.001 {
            // Fly in the upper quarter of the scene, at least 3 cells below the top
            let max_y = (th / 4).max(3);
            let y = 3.0 + (rng.random::<u16>() % max_y) as f32;
            let speed = 0.28 + rng.random::<f32>() * 0.18;
            // Start to the left so the plane flies in from off-screen
            self.planes.push(Airplane { x: -14.0, y, speed });
            self.spawn_cooldown = 500 + (rng.random::<u16>() % 400);
        }
    }

    pub fn render(&self, canvas: &mut HalfBlockCanvas, dark_bg: bool) {
        let (body_color, wing_color, engine_color, window_color, trail_color) = if dark_bg {
            (
                Color::White,
                Color::Rgb(190, 210, 230),
                Color::Rgb(120, 140, 160),
                Color::Rgb(100, 200, 255),
                Color::Rgb(160, 160, 180),
            )
        } else {
            (
                Color::Rgb(50, 50, 70),
                Color::Rgb(70, 80, 100),
                Color::Rgb(80, 80, 100),
                Color::Rgb(0, 80, 150),
                Color::Rgb(130, 130, 150),
            )
        };

        for plane in &self.planes {
            render_plane(
                canvas,
                plane.x,
                plane.y,
                body_color,
                wing_color,
                engine_color,
                window_color,
                trail_color,
            );
        }
    }
}

/// Draw a side-view commercial-jet silhouette flying right (nose on the right).
///
/// Layout at (x, y) in cell float coordinates:
///
///  contrail   tail  fuselage body    nose
///  ──── ··· ──┬──────────────────────>
///              │ H-stab         wings
///
fn render_plane(
    canvas: &mut HalfBlockCanvas,
    x: f32,
    y: f32,
    body: Color,
    wing: Color,
    engine: Color,
    window: Color,
    trail: Color,
) {
    // ── Fuselage ──────────────────────────────────────────────────────────────
    // Centre spine
    canvas.draw_line(x - 5.0, y, x + 5.5, y, body);
    // Upper skin
    canvas.draw_line(x - 4.5, y - 0.5, x + 4.5, y - 0.5, body);
    // Lower skin
    canvas.draw_line(x - 4.5, y + 0.5, x + 4.5, y + 0.5, body);

    // ── Nose cone ─────────────────────────────────────────────────────────────
    canvas.draw_line(x + 4.5, y - 0.5, x + 5.5, y, body);
    canvas.draw_line(x + 4.5, y + 0.5, x + 5.5, y, body);

    // ── Main wings (swept-back, flying right) ─────────────────────────────────
    // The wings root near x+1, tips sweep back to x-2 and span ±2.5 cells tall.
    // Top wing
    canvas.draw_line(x + 1.5, y - 0.5, x - 1.5, y - 2.5, wing); // leading edge
    canvas.draw_line(x + 0.0, y - 0.5, x - 2.5, y - 2.5, wing); // trailing edge
    canvas.draw_line(x - 1.5, y - 2.5, x - 2.5, y - 2.5, wing); // tip edge
    // Bottom wing (mirror)
    canvas.draw_line(x + 1.5, y + 0.5, x - 1.5, y + 2.5, wing); // leading edge
    canvas.draw_line(x + 0.0, y + 0.5, x - 2.5, y + 2.5, wing); // trailing edge
    canvas.draw_line(x - 1.5, y + 2.5, x - 2.5, y + 2.5, wing); // tip edge

    // ── Tail section ──────────────────────────────────────────────────────────
    // Vertical stabiliser (sweeps back and upward from the tailcone)
    canvas.draw_line(x - 4.0, y - 0.5, x - 5.0, y - 2.0, wing);
    canvas.draw_line(x - 3.5, y - 0.5, x - 4.8, y - 2.0, wing);
    // Horizontal stabilisers (small, swept like the main wings but narrower)
    canvas.draw_line(x - 3.8, y - 0.5, x - 5.0, y - 1.2, wing); // top stab
    canvas.draw_line(x - 3.8, y + 0.5, x - 5.0, y + 1.2, wing); // bottom stab

    // ── Engine pods (below the wings, centred at ~x+0.5) ─────────────────────
    canvas.draw_line(x + 0.5, y + 1.2, x - 0.8, y + 1.2, engine);
    canvas.plot_f(x + 0.7, y + 1.5, engine); // intake
    canvas.plot_f(x - 1.0, y + 1.5, engine); // nozzle

    // ── Windows ───────────────────────────────────────────────────────────────
    // A row of 5 window dots along the upper-cabin line
    for i in 0..5i32 {
        canvas.plot_f(x + 3.5 - i as f32 * 0.9, y - 0.5, window);
    }

    // ── Contrail ──────────────────────────────────────────────────────────────
    // Fading dotted line from the tail backward
    let trail_steps = 8;
    for i in 1..=trail_steps {
        let tx = x - 5.0 - i as f32 * 0.9;
        // Fade: draw every other dot in the second half
        if i <= trail_steps / 2 || i % 2 == 0 {
            canvas.plot_f(tx, y, trail);
        }
    }
}
