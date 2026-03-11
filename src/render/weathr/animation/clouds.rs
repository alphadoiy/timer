use crate::render::weathr::HalfBlockCanvas;
use rand::prelude::*;
use ratatui::style::Color;

/// A single cloud: one flat body ellipse + large overlapping top lobes.
///
/// # Coordinate convention (cell coords, terminal cells are ~2:1 tall)
///
/// To get a visually circular shape with `fill_ellipse(cx, cy, rx, ry)`:
///   visual_width  ≈ rx * cw
///   visual_height ≈ ry * ch = ry * 2 * cw
///   ∴ visually round  → ry = rx / 2
///     2:1 wide (flat) → ry = rx / 4
///
/// The body uses ry_body = rx / 6 (flat bottom edge).
/// Each lobe uses ry_lobe = br / 2 (visually round puffs).
/// Lobes are large (proportional to rx) and overlap, forming a smooth
/// classic cloud silhouette like the reference image.
struct Cloud {
    x: f32,
    y: f32,
    speed: f32,
    wind_x: f32,
    /// Horizontal semi-radius of the flat body (cells).
    rx: f32,
    /// Vertical semi-radius of the flat body (cells).
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
        Self {
            clouds,
            terminal_width: tw,
            terminal_height: th,
            base_wind_x,
            next_id,
        }
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
        let speed = 0.025 + rng.random::<f32>() * 0.035;
        let wind_x = base_wind * (0.7 + rng.random::<f32>() * 0.6);

        let max_rx = (tw as f32 * 0.09).max(5.5).min(12.0);
        let rx = 4.5 + rng.random::<f32>() * (max_rx - 4.5);
        // Very flat body forms the cloud's flat bottom edge.
        let ry_body = rx / 6.0;

        // Tallest lobe (center) reaches ~rx*0.35 above center; keep inside canvas.
        let bump_clearance = (rx * 0.35).ceil().max(1.0);
        let y_upper = (th as f32 * 0.40).max(bump_clearance + 2.0);
        let y = bump_clearance + rng.random::<f32>() * (y_upper - bump_clearance);

        // Classic cloud silhouette: 3 large overlapping lobes + 2 side puffs.
        // Lobe radii scale with rx so they merge into a smooth top contour.
        let mut bumps = Vec::with_capacity(5);
        let jx = (rng.random::<f32>() - 0.5) * rx * 0.04;

        // Center lobe — tallest.
        let cr = rx * (0.33 + rng.random::<f32>() * 0.06);
        bumps.push((jx, -(cr * 0.35), cr));

        // Left lobe.
        let lr = rx * (0.26 + rng.random::<f32>() * 0.05);
        bumps.push((-rx * (0.36 + rng.random::<f32>() * 0.04), -(lr * 0.25), lr));

        // Right lobe.
        let rr = rx * (0.26 + rng.random::<f32>() * 0.05);
        bumps.push((rx * (0.36 + rng.random::<f32>() * 0.04), -(rr * 0.25), rr));

        // Left edge puff.
        let lp = rx * (0.16 + rng.random::<f32>() * 0.03);
        bumps.push((
            -rx * (0.68 + rng.random::<f32>() * 0.04),
            -ry_body * 0.05,
            lp,
        ));

        // Right edge puff.
        let rp = rx * (0.15 + rng.random::<f32>() * 0.03);
        bumps.push((
            rx * (0.68 + rng.random::<f32>() * 0.04),
            -ry_body * 0.05,
            rp,
        ));

        Cloud {
            x,
            y,
            speed,
            wind_x,
            rx,
            ry_body,
            bumps,
            is_dark,
        }
    }

    pub fn update(
        &mut self,
        tw: u16,
        th: u16,
        is_clear: bool,
        use_dark_palette: bool,
        rng: &mut impl Rng,
    ) {
        self.terminal_width = tw;
        self.terminal_height = th;

        for c in &mut self.clouds {
            c.x += c.speed + c.wind_x;
            c.is_dark = use_dark_palette;
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
            self.clouds.push(Self::make_cloud(
                0.0,
                tw,
                th,
                use_dark_palette,
                self.base_wind_x,
                rng,
            ));
            self.next_id = self.next_id.wrapping_add(1);
        }
    }

    pub fn render(&self, canvas: &mut HalfBlockCanvas, dark_bg: bool) {
        for cloud in &self.clouds {
            let body_color = if cloud.is_dark {
                if dark_bg {
                    Color::Rgb(148, 156, 176)
                } else {
                    Color::Rgb(132, 142, 160)
                }
            } else if dark_bg {
                Color::Rgb(210, 210, 220)
            } else {
                Color::Rgb(122, 130, 146)
            };
            let highlight_color = if cloud.is_dark {
                if dark_bg {
                    Color::Rgb(172, 182, 202)
                } else {
                    Color::Rgb(156, 168, 186)
                }
            } else if dark_bg {
                Color::Rgb(232, 232, 242)
            } else {
                Color::Rgb(142, 152, 170)
            };
            let shadow_color = if cloud.is_dark {
                if dark_bg {
                    Color::Rgb(118, 126, 144)
                } else {
                    Color::Rgb(108, 116, 132)
                }
            } else if dark_bg {
                Color::Rgb(160, 162, 175)
            } else {
                Color::Rgb(116, 124, 140)
            };

            // Flat body base — full width, shifted slightly down for a flat bottom.
            canvas.fill_ellipse(
                cloud.x,
                cloud.y + cloud.ry_body * 0.10,
                cloud.rx * 0.97,
                cloud.ry_body,
                body_color,
            );
            canvas.fill_ellipse(
                cloud.x,
                cloud.y,
                cloud.rx * 0.93,
                cloud.ry_body * 0.90,
                body_color,
            );

            // Soft bottom shadow.
            canvas.fill_ellipse(
                cloud.x,
                cloud.y + cloud.ry_body * 0.70,
                cloud.rx * 0.75,
                cloud.ry_body * 0.28,
                shadow_color,
            );
            canvas.dither_ellipse(
                cloud.x,
                cloud.y + cloud.ry_body * 0.55,
                cloud.rx * 0.78,
                cloud.ry_body * 0.34,
                0.38,
                shadow_color,
                (cloud.x.to_bits() ^ cloud.y.to_bits()).wrapping_add(17),
            );

            // Large overlapping lobes form the puffy top.
            for (i, &(bx, by, br)) in cloud.bumps.iter().enumerate() {
                let lobe_ry = br / 2.0;
                canvas.fill_ellipse(cloud.x + bx, cloud.y + by, br, lobe_ry, body_color);
                let hx = cloud.x + bx - br * 0.10;
                let hy = cloud.y + by - lobe_ry * 0.20;
                canvas.fill_ellipse(hx, hy, br * 0.50, lobe_ry * 0.40, highlight_color);
                canvas.dither_ellipse(
                    hx,
                    hy,
                    br * 0.56,
                    lobe_ry * 0.52,
                    0.32,
                    highlight_color,
                    (i as u32).wrapping_mul(97).wrapping_add(cloud.x.to_bits()),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CloudSystem;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn generated_cloud_has_classic_silhouette() {
        let mut rng = StdRng::seed_from_u64(42);
        let cloud = CloudSystem::make_cloud(10.0, 120, 34, false, 0.12, &mut rng);
        assert_eq!(cloud.bumps.len(), 5, "3 lobes + 2 puffs");
        assert!(cloud.rx >= 4.5 && cloud.rx <= 12.0);
        assert!(cloud.ry_body < cloud.rx / 5.0, "body should be flat");
        for &(_bx, by, br) in &cloud.bumps {
            let bump_top = cloud.y + by - br / 2.0;
            assert!(bump_top >= 0.0, "lobe overflows top: {bump_top:.2}");
        }
    }
}
