use std::f32::consts::{FRAC_PI_2, TAU};

use super::*;

pub(super) fn render_clock_visual_panel(
    area: Rect,
    buf: &mut Buffer,
    pose: SpritePose,
    theme: Theme,
    clock: &ClockSnapshot,
) {
    let panes = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(78), Constraint::Percentage(22)])
        .split(area);
    render_braille_clock_dial(panes[0], buf, pose, theme, clock);
    super::render_figlet_time(panes[1], buf, &clock.time_text, theme);
}

fn render_braille_clock_dial(
    area: Rect,
    buf: &mut Buffer,
    pose: SpritePose,
    theme: Theme,
    clock: &ClockSnapshot,
) {
    let mut canvas = BrailleCanvas::new(area);
    let center_x = canvas.width_sub as f32 / 2.0 + pose.dial_offset_x as f32 * 2.0;
    let center_y = canvas.height_sub as f32 / 2.0 + pose.dial_offset_y as f32 * 4.0;
    let radius =
        (canvas.width_sub.min(canvas.height_sub) as f32 * 0.40 * pose.radius_scale).max(8.0);

    canvas.draw_ellipse(
        center_x,
        center_y,
        radius + 0.35,
        radius + 0.35,
        theme.outline,
        1.8,
    );
    canvas.draw_ellipse(
        center_x,
        center_y,
        radius - 0.35,
        radius - 0.35,
        theme.outline,
        1.8,
    );
    canvas.draw_ellipse(
        center_x,
        center_y,
        radius - 2.0,
        radius - 2.0,
        theme.accent_soft,
        4.0,
    );
    draw_dial_ticks(
        &mut canvas,
        center_x,
        center_y,
        radius,
        theme.subtext,
        theme.outline,
    );

    let second_turn = clock.second_angle + pose.second_sweep + pose.tilt;
    draw_hour_hand(
        &mut canvas,
        center_x,
        center_y,
        radius * 0.50,
        clock.hour_angle + pose.hour_sweep + pose.tilt,
        theme.hour_hand,
    );
    draw_minute_hand(
        &mut canvas,
        center_x,
        center_y,
        radius * 0.78,
        clock.minute_angle + pose.minute_sweep + pose.tilt,
        theme.minute_hand,
    );
    draw_second_hand(
        &mut canvas,
        center_x,
        center_y,
        radius * 0.93,
        second_turn,
        theme.second_hand,
    );

    canvas.render(buf);
    super::put(
        buf,
        sub_to_cell_x(area, center_x),
        sub_to_cell_y(area, center_y),
        "●",
        theme.accent,
    );
    overlay_dial_labels(buf, area, center_x, center_y, radius, theme);
}

fn draw_dial_ticks(
    canvas: &mut BrailleCanvas,
    cx: f32,
    cy: f32,
    radius: f32,
    mark: ratatui::style::Color,
    cardinal: ratatui::style::Color,
) {
    for i in 0..60 {
        let t = i as f32 / 60.0;
        let theta = t * TAU - FRAC_PI_2;
        let c = theta.cos();
        let s = theta.sin();
        let outer_x = cx + c * radius;
        let outer_y = cy + s * radius;
        let inner_scale = if i % 5 == 0 { 0.80 } else { 0.92 };
        let inner_x = cx + c * radius * inner_scale;
        let inner_y = cy + s * radius * inner_scale;
        if i % 15 == 0 {
            draw_thick_line(canvas, inner_x, inner_y, outer_x, outer_y, 1.6, cardinal);
        } else if i % 5 == 0 {
            draw_thick_line(canvas, inner_x, inner_y, outer_x, outer_y, 1.4, cardinal);
        } else {
            canvas.draw_line(inner_x, inner_y, outer_x, outer_y, mark);
        }
    }
}

fn draw_hour_hand(
    canvas: &mut BrailleCanvas,
    cx: f32,
    cy: f32,
    length: f32,
    turn: f32,
    core: ratatui::style::Color,
) {
    let theta = turn * TAU - FRAC_PI_2;
    let dx = theta.cos();
    let dy = theta.sin();
    let nx = -dy;
    let ny = dx;
    let shaft_end = length * 0.90;
    draw_thick_line(
        canvas,
        cx,
        cy,
        cx + dx * shaft_end,
        cy + dy * shaft_end,
        3.2,
        core,
    );
    let (tip_x, tip_y) = hand_endpoint(cx, cy, length, turn);
    draw_thick_line(
        canvas,
        tip_x - nx * 1.35,
        tip_y - ny * 1.35,
        tip_x + nx * 1.35,
        tip_y + ny * 1.35,
        0.35,
        core,
    );
}

fn draw_minute_hand(
    canvas: &mut BrailleCanvas,
    cx: f32,
    cy: f32,
    length: f32,
    turn: f32,
    core: ratatui::style::Color,
) {
    let theta = turn * TAU - FRAC_PI_2;
    let dx = theta.cos();
    let dy = theta.sin();
    let nx = -dy;
    let ny = dx;
    let shaft_end = length * 0.94;
    draw_thick_line(
        canvas,
        cx,
        cy,
        cx + dx * shaft_end,
        cy + dy * shaft_end,
        1.05,
        core,
    );
    let (tip_x, tip_y) = hand_endpoint(cx, cy, length, turn);
    let (shaft_x, shaft_y) = hand_endpoint(cx, cy, shaft_end, turn);
    draw_thick_line(
        canvas,
        shaft_x - nx * 0.9,
        shaft_y - ny * 0.9,
        tip_x,
        tip_y,
        0.0,
        core,
    );
    draw_thick_line(
        canvas,
        shaft_x + nx * 0.9,
        shaft_y + ny * 0.9,
        tip_x,
        tip_y,
        0.0,
        core,
    );
}

fn draw_second_hand(
    canvas: &mut BrailleCanvas,
    cx: f32,
    cy: f32,
    length: f32,
    turn: f32,
    color: ratatui::style::Color,
) {
    let theta = turn * TAU - FRAC_PI_2;
    let dx = theta.cos();
    let dy = theta.sin();
    let nx = -dy;
    let ny = dx;
    let steps = length.ceil().max(1.0) as i32;

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let px = cx + dx * length * t;
        let py = cy + dy * length * t;
        let shade = lerp_color(scale_color(color, 0.42), color, t);
        canvas.plot(px.round() as i32, py.round() as i32, shade);

        let aa = scale_color(color, 0.18);
        canvas.plot(
            (px + nx * 0.8).round() as i32,
            (py + ny * 0.8).round() as i32,
            aa,
        );
        canvas.plot(
            (px - nx * 0.8).round() as i32,
            (py - ny * 0.8).round() as i32,
            aa,
        );
    }

    let (tip_x, tip_y) = hand_endpoint(cx, cy, length + 1.2, turn);
    canvas.plot(tip_x.round() as i32, tip_y.round() as i32, color);

    let (tail_x, tail_y) = hand_endpoint(cx, cy, -length * 0.16, turn);
    canvas.draw_line(cx, cy, tail_x, tail_y, scale_color(color, 0.55));

    canvas.plot(tail_x.round() as i32, tail_y.round() as i32, color);
    canvas.plot((tail_x + 1.0).round() as i32, tail_y.round() as i32, color);
    canvas.plot(tail_x.round() as i32, (tail_y + 1.0).round() as i32, color);
    canvas.plot(
        (tail_x + 1.0).round() as i32,
        (tail_y + 1.0).round() as i32,
        color,
    );
}

fn hand_endpoint(cx: f32, cy: f32, length: f32, turn: f32) -> (f32, f32) {
    let theta = turn * TAU - FRAC_PI_2;
    (cx + theta.cos() * length, cy + theta.sin() * length)
}

fn overlay_dial_labels(
    buf: &mut Buffer,
    area: Rect,
    center_x: f32,
    center_y: f32,
    radius: f32,
    theme: Theme,
) {
    let labels = [
        ("12", -FRAC_PI_2),
        ("3", 0.0),
        ("6", FRAC_PI_2),
        ("9", std::f32::consts::PI),
    ];
    for (label, theta) in labels {
        let x = center_x + theta.cos() * radius * 0.68;
        let y = center_y + theta.sin() * radius * 0.68;
        let cell_x = sub_to_cell_x(area, x);
        let cell_y = sub_to_cell_y(area, y);
        if label == "12" || label == "6" {
            super::put_centered(
                buf,
                Rect::new(area.x, cell_y as u16, area.width, 1),
                cell_y,
                label,
                theme.accent_soft,
            );
        } else {
            super::put(buf, cell_x, cell_y, label, theme.accent_soft);
        }
    }
}

fn sub_to_cell_x(area: Rect, x_sub: f32) -> i16 {
    area.x as i16 + (x_sub / 2.0).round() as i16
}

fn sub_to_cell_y(area: Rect, y_sub: f32) -> i16 {
    area.y as i16 + (y_sub / 4.0).round() as i16
}

fn scale_color(color: ratatui::style::Color, factor: f32) -> ratatui::style::Color {
    match color {
        ratatui::style::Color::Rgb(r, g, b) => ratatui::style::Color::Rgb(
            ((r as f32) * factor).clamp(0.0, 255.0) as u8,
            ((g as f32) * factor).clamp(0.0, 255.0) as u8,
            ((b as f32) * factor).clamp(0.0, 255.0) as u8,
        ),
        _ => color,
    }
}

fn lerp_color(
    from: ratatui::style::Color,
    to: ratatui::style::Color,
    t: f32,
) -> ratatui::style::Color {
    match (from, to) {
        (ratatui::style::Color::Rgb(fr, fg, fb), ratatui::style::Color::Rgb(tr, tg, tb)) => {
            let t = t.clamp(0.0, 1.0);
            ratatui::style::Color::Rgb(
                (fr as f32 + (tr as f32 - fr as f32) * t).round() as u8,
                (fg as f32 + (tg as f32 - fg as f32) * t).round() as u8,
                (fb as f32 + (tb as f32 - fb as f32) * t).round() as u8,
            )
        }
        _ => to,
    }
}

fn draw_thick_line(
    canvas: &mut BrailleCanvas,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    thickness: f32,
    color: ratatui::style::Color,
) {
    if thickness <= 0.0 {
        canvas.draw_line(x0, y0, x1, y1, color);
        return;
    }

    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let nx = -dy / len;
    let ny = dx / len;
    let steps = thickness.round() as i32;
    for offset in -steps..=steps {
        let shift = offset as f32 * 0.7;
        canvas.draw_line(
            x0 + nx * shift,
            y0 + ny * shift,
            x1 + nx * shift,
            y1 + ny * shift,
            color,
        );
    }
}
