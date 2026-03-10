use crate::render::weathr::BrailleWeatherCanvas;
use rand::prelude::*;
use ratatui::style::Color;

/// A single cloud made of one flat body ellipse and several rounded bumps.
///
/// # Coordinate convention (cell coords, terminal cells are ~2:1 tall)
///
/// To get a visually circular shape with `fill_ellipse(cx, cy, rx, ry)`:
///   visual_width  ≈ rx * cw
///   visual_height ≈ ry * ch = ry * 2 * cw
///   ∴ visually round  → ry = rx / 2
///     2:1 wide (flat) → ry = rx / 4
///
/// The body uses ry_body = rx / 4 (nice flat base).
/// Each bump uses ry_bump = br / 2 (visually round puffs).
struct Cloud {
    x: f32,
    y: f32,
    speed: f32,
    wind_x: f32,
    /// Horizontal semi-radius of the flat body (cells).
    rx: f32,
    /// Vertical semi-radius of the flat body (cells) = rx / 4.
    ry_body: f32,
    /// List of (x_offset, y_offset, radius) for the top puffs.
    bumps: Vec<(f32, f32, f32)>,
    is_dark: bool,
}

pub struct CloudSystem {
    clouds: Vec<Cloud>,
    terminal_width: u16,
    terminal_height: u16,
    base_wind_x: f32,
    next_id: u32,
}

impl CloudSystem {
    pub fn new(tw: u16, th: u16) -> Self {
        let mut rng = rand::rng();
        let base_wind_x = 0.12;
        let count = (tw / 35).max(1) as usize;
        let segment = tw as f32 / count as f32;
        let mut clouds = Vec::with_capacity(count + 2);
        let mut next_id = 1u32;
        for i in 0..count {
            let x_min = (i as f32 * segment) as u16;
            let x_max = ((i as f32 + 1.0) * segment) as u16;
            let x = rng.random_range(x_min..=x_max) as f32;
            clouds.push(Self::make_cloud(x, tw, th, false, base_wind_x, &mut rng));
            next_id = next_id.wrapping_add(1);
        }
        Self { clouds, terminal_width: tw, terminal_height: th, base_wind_x, next_id }
    }

    pub fn set_wind(&mut self, speed_kmh: f32, direction_deg: f32) {
        let rad = direction_deg.to_radians();
        self.base_wind_x = (speed_kmh / 50.0) * (-rad.sin());
        let mut rng = rand::rng();
        for c in &mut self.clouds {
            c.wind_x = self.base_wind_x * (0.8 + rng.random::<f32>() * 0.4);
        }
    }

    fn make_cloud(
        x: f32,
        tw: u16,
        th: u16,
        is_dark: bool,
        base_wind: f32,
        rng: &mut impl Rng,
    ) -> Cloud {
        // Clouds live in the top 40% of the scene.
        let y_max = ((th as f32 * 0.38).ceil() as u16).max(2);
        let y = 1.0 + (rng.random::<u16>() % y_max) as f32;
        let speed = 0.025 + rng.random::<f32>() * 0.035;
        let wind_x = base_wind * (0.7 + rng.random::<f32>() * 0.6);
        // Width varies with terminal width so clouds look proportional.
        let max_rx = (tw as f32 * 0.08).max(4.0).min(10.0);
        let rx = 3.5 + rng.random::<f32>() * (max_rx - 3.5);
        // Flat body: ry = rx/4 gives 2:1 visual aspect ratio.
        let ry_body = rx / 4.0;

        // Generate 2–4 rounded bumps evenly across the top of the body.
        let bump_count = 2 + (rng.random::<u32>() % 3) as usize;
        let mut bumps = Vec::with_capacity(bump_count);
        // Spread bumps across [-rx*0.65, rx*0.65] with slight jitter.
        let span = rx * 1.3;
        let step = span / bump_count as f32;
        for i in 0..bump_count {
            let bx = -rx * 0.65 + step * (i as f32 + 0.5)
                + (rng.random::<f32>() - 0.5) * step * 0.3;
            // Bump radius: smaller near the edges, taller in the middle.
            let edge_factor = 1.0 - (2.0 * i as f32 / (bump_count - 1).max(1) as f32 - 1.0).abs();
            let br = ry_body * 1.2 + edge_factor * ry_body * 1.5 + rng.random::<f32>() * ry_body;
            // y-offset: bump circle sits on top of the body (just above the body edge).
            // body top is at cy - ry_body. bump center: cy - ry_body - br*0.5 + overlap
            let overlap = br * 0.4; // overlap with body for seamless look
            let by = -(ry_body + br / 2.0 - overlap);
            bumps.push((bx, by, br));
        }

        Cloud { x, y, speed, wind_x, rx, ry_body, bumps, is_dark }
    }

    pub fn update(&mut self, tw: u16, th: u16, is_clear: bool, rng: &mut impl Rng) {
        self.terminal_width = tw;
        self.terminal_height = th;

        for c in &mut self.clouds {
            c.x += c.speed + c.wind_x;
            // Wrap: when the cloud clears the right edge, re-enter from the left.
            if c.x - c.rx > tw as f32 {
                c.x = -(c.rx);
            }
        }

        let max_clouds = if is_clear {
            (tw / 35).max(1) as usize
        } else {
            (tw / 22).max(2) as usize
        };
        let spawn_chance: f32 = if is_clear { 0.003 } else { 0.007 };

        let min_gap = (tw as f32 / 6.0).max(12.0);
        let too_close = self.clouds.iter().any(|c| c.x < min_gap && c.x > 0.0);
        if self.clouds.len() < max_clouds && !too_close && rng.random::<f32>() < spawn_chance {
            self.clouds.push(Self::make_cloud(0.0, tw, th, !is_clear, self.base_wind_x, rng));
            self.next_id = self.next_id.wrapping_add(1);
        }
    }

    pub fn render_braille(&self, canvas: &mut BrailleWeatherCanvas, dark_bg: bool) {
        for cloud in &self.clouds {
            let body_color = if cloud.is_dark {
                if dark_bg { Color::Rgb(90, 90, 100) } else { Color::Rgb(120, 120, 130) }
            } else if dark_bg {
                Color::Rgb(210, 210, 220)
            } else {
                Color::Rgb(75, 75, 85)
            };
            let shadow_color = if cloud.is_dark {
                if dark_bg { Color::Rgb(70, 70, 80) } else { Color::Rgb(100, 100, 110) }
            } else if dark_bg {
                Color::Rgb(160, 162, 175)
            } else {
                Color::Rgb(100, 100, 110)
            };

            // ── Flat body base ───────────────────────────────────────────────
            // The body is a wide flat ellipse: 2:1 visual aspect ratio.
            canvas.fill_ellipse(cloud.x, cloud.y, cloud.rx, cloud.ry_body, body_color);

            // Thin shadow line on the bottom edge of the body.
            canvas.fill_ellipse(
                cloud.x,
                cloud.y + cloud.ry_body * 0.6,
                cloud.rx * 0.85,
                cloud.ry_body * 0.35,
                shadow_color,
            );

            // ── Rounded top puffs ────────────────────────────────────────────
            // Each bump is a visually round ellipse (ry = br/2 → 1:1 visual).
            for &(bx, by, br) in &cloud.bumps {
                let bump_ry = br / 2.0;
                canvas.fill_ellipse(cloud.x + bx, cloud.y + by, br, bump_ry, body_color);
            }
        }
    }
}
