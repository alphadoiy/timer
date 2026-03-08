use std::{
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crossterm::style::Color as CtColor;

use crate::render::weathr::{
    TerminalRenderer,
    animation::{
        airplanes::AirplaneSystem, birds::BirdSystem, chimney::ChimneySmoke, clouds::CloudSystem,
        fireflies::FireflySystem, fog::FogSystem, leaves::FallingLeaves, moon::MoonSystem,
        raindrops::RaindropSystem, snow::SnowSystem, stars::StarSystem,
        thunderstorm::ThunderstormSystem,
    },
    scene::house::House,
    types::{FogIntensity, RainIntensity, SnowIntensity},
};

use super::*;

const SIM_STEP: Duration = Duration::from_millis(33);
const MAX_SIM_STEPS_PER_FRAME: u8 = 8;

type RenderPass = fn(&mut WeatherScene, RenderQuality, bool);

const RENDER_PIPELINE: [RenderPass; 4] = [
    WeatherScene::render_sky_layer,
    WeatherScene::render_mid_layer,
    WeatherScene::render_ground_layer,
    WeatherScene::render_fx_layer,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WeatherPreset {
    Sunny,
    Rainy,
    Snowy,
    Foggy,
    Stormy,
}

impl WeatherPreset {
    fn from_state(running: bool, finish_sprint: bool, cycle: u32) -> Self {
        if finish_sprint {
            return Self::Stormy;
        }
        if !running {
            return Self::Foggy;
        }
        match cycle % 4 {
            1 => Self::Sunny,
            2 => Self::Rainy,
            3 => Self::Snowy,
            _ => Self::Sunny,
        }
    }

    fn is_rainy(self) -> bool {
        matches!(self, Self::Rainy | Self::Stormy)
    }

    fn is_snowy(self) -> bool {
        matches!(self, Self::Snowy)
    }

    fn is_foggy(self) -> bool {
        matches!(self, Self::Foggy | Self::Stormy)
    }

    fn has_thunder(self) -> bool {
        matches!(self, Self::Stormy)
    }
}

#[derive(Clone, Copy)]
struct RenderQuality {
    show_mountains: bool,
    show_house: bool,
    show_birds: bool,
    show_airplanes: bool,
    show_fireflies: bool,
    show_fog: bool,
    show_snow: bool,
    show_thunderstorm: bool,
    show_flowers: bool,
}

impl RenderQuality {
    fn for_size(width: u16, height: u16) -> Self {
        Self {
            show_mountains: width >= 48 && height >= 11,
            show_house: width >= 50 && height >= 12,
            show_birds: width >= 52 && height >= 12,
            show_airplanes: width >= 80 && height >= 16,
            show_fireflies: width >= 55 && height >= 12,
            show_fog: width >= 50 && height >= 12,
            show_snow: width >= 60 && height >= 13,
            show_thunderstorm: width >= 70 && height >= 14,
            show_flowers: width >= 45 && height >= 10,
        }
    }
}

pub(super) fn render_pomodoro_road_panel(
    area: Rect,
    buf: &mut Buffer,
    pomodoro: PomodoroSnapshot,
    _pose: SpritePose,
    theme: Theme,
) {
    let remaining_secs = pomodoro.remaining.as_secs();
    let countdown = super::format_duration(pomodoro.remaining);
    let finish_sprint = pomodoro.running && remaining_secs <= 10 && !pomodoro.completed;
    let now = phase_seconds();
    let (jx, jy) = if finish_sprint {
        (
            ((now * 48.0).sin() * 1.4).round() as i16,
            ((now * 37.0).cos() * 0.8).round() as i16,
        )
    } else {
        (0, 0)
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(32), Constraint::Percentage(68)])
        .split(area);
    render_pomodoro_countdown(rows[0], buf, &countdown, jx, jy);

    let road_area = rows[1];
    let road_block = Block::default()
        .title(Line::from(Span::styled(
            " Focus Road ",
            Style::default()
                .fg(theme.accent_soft)
                .add_modifier(Modifier::BOLD),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.outline));
    let inner = road_block.inner(road_area);
    road_block.render(road_area, buf);

    if inner.width < 24 || inner.height < 10 {
        Paragraph::new(Line::from(Span::styled(
            "terminal too small for animation",
            Style::default().fg(theme.subtext),
        )))
        .alignment(Alignment::Center)
        .render(inner, buf);
        return;
    }

    let lanes = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(6), Constraint::Length(1)])
        .split(inner);
    let top = lanes[0];
    let world = lanes[1];
    let bottom = lanes[2];

    super::put_text(buf, top.x as i16, top.y as i16, "25:00", ratatui::style::Color::Black);
    let right_x = top.right() as i16 - UnicodeWidthStr::width("00:00") as i16;
    super::put_text(
        buf,
        right_x,
        top.y as i16,
        "00:00",
        ratatui::style::Color::Black,
    );
    super::put_text(
        buf,
        (right_x - 6).max(top.x as i16),
        top.y as i16,
        "FIN",
        ratatui::style::Color::Rgb(210, 150, 0),
    );

    render_weathr_animation(world, buf, pomodoro.running, pomodoro.cycle, finish_sprint);

    Paragraph::new(Line::from(Span::styled(
        if pomodoro.running {
            "Focus: full weathr scene"
        } else {
            "Paused: idling scene"
        },
        Style::default().fg(theme.subtext),
    )))
    .alignment(Alignment::Center)
    .render(bottom, buf);
}

fn render_weathr_animation(
    area: Rect,
    buf: &mut Buffer,
    running: bool,
    cycle: u32,
    finish_sprint: bool,
) {
    let state = weather_state();
    let mut state = state.lock().expect("weather animation mutex poisoned");
    state.update(area.width, area.height, running, cycle, finish_sprint);
    state.render(area, buf, finish_sprint);
}

struct WeatherScene {
    renderer: TerminalRenderer,
    house: House,
    chimney: ChimneySmoke,
    clouds: CloudSystem,
    rain: RaindropSystem,
    snow: SnowSystem,
    fog: FogSystem,
    stars: StarSystem,
    moon: MoonSystem,
    birds: BirdSystem,
    airplanes: AirplaneSystem,
    leaves: FallingLeaves,
    fireflies: FireflySystem,
    thunderstorm: ThunderstormSystem,
    frame: u64,
    last_update: Instant,
    accumulator: Duration,
    quality: RenderQuality,
    preset: WeatherPreset,
}

impl WeatherScene {
    fn new(width: u16, height: u16) -> Self {
        Self {
            renderer: TerminalRenderer::new(width, height),
            house: House,
            chimney: ChimneySmoke::new(),
            clouds: CloudSystem::new(width, height),
            rain: RaindropSystem::new(width, height, RainIntensity::Light),
            snow: SnowSystem::new(width, height, SnowIntensity::Light),
            fog: FogSystem::new(width, height, FogIntensity::Light),
            stars: StarSystem::new(width, height),
            moon: MoonSystem::new(width, height, Some(0.45)),
            birds: BirdSystem::new(width, height),
            airplanes: AirplaneSystem::new(width, height),
            leaves: FallingLeaves::new(width, height),
            fireflies: FireflySystem::new(width, height),
            thunderstorm: ThunderstormSystem::new(width, height),
            frame: 0,
            last_update: Instant::now(),
            accumulator: Duration::ZERO,
            quality: RenderQuality::for_size(width, height),
            preset: WeatherPreset::Stormy,
        }
    }

    fn update(&mut self, width: u16, height: u16, running: bool, cycle: u32, finish_sprint: bool) {
        self.renderer.resize(width, height);
        self.quality = RenderQuality::for_size(width, height);
        self.apply_preset(WeatherPreset::from_state(running, finish_sprint, cycle));

        let now = Instant::now();
        let delta = now
            .saturating_duration_since(self.last_update)
            .min(Duration::from_millis(250));
        self.last_update = now;
        self.accumulator = self.accumulator.saturating_add(delta);

        let mut steps = 0u8;
        while self.accumulator >= SIM_STEP && steps < MAX_SIM_STEPS_PER_FRAME {
            self.advance_simulation(width, height);
            self.accumulator = self.accumulator.saturating_sub(SIM_STEP);
            steps = steps.saturating_add(1);
        }

        if self.frame == 0 {
            self.advance_simulation(width, height);
        }
    }

    fn advance_simulation(&mut self, width: u16, height: u16) {
        self.frame = self.frame.wrapping_add(1);

        let mut rng = rand::rng();
        let is_day = true;

        self.clouds.set_cloud_color(is_day);
        self.clouds.set_wind(14.0, 255.0);
        self.clouds
            .update(width, height, is_day, CtColor::White, &mut rng);

        if self.preset.is_rainy() {
            self.rain.update(width, height, &mut rng);
        }

        if self.quality.show_snow && self.preset.is_snowy() {
            self.snow.update(width, height, &mut rng);
        }

        if self.quality.show_fog && self.preset.is_foggy() {
            self.fog.update(width, height, &mut rng);
        }

        self.stars.update(width, height, &mut rng);
        self.moon.set_phase(((self.frame % 1200) as f64) / 1200.0);
        self.moon.update(width, height);

        if self.quality.show_birds {
            self.birds.update(width, height, &mut rng);
        }
        if self.quality.show_airplanes {
            self.airplanes.update(width, height, &mut rng);
        }

        self.leaves.update(width, height, &mut rng);

        if self.quality.show_fireflies {
            let horizon = height.saturating_sub(6);
            self.fireflies.update(width, height, horizon, &mut rng);
        }

        if self.quality.show_thunderstorm && self.preset.has_thunder() {
            self.thunderstorm.update(width, height, &mut rng);
        }

        if self.quality.show_house {
            let house_x = width.saturating_sub(House::WIDTH).saturating_sub(2);
            let house_y = height.saturating_sub(House::HEIGHT).saturating_sub(2);
            let chimney_x = house_x.saturating_add(House::CHIMNEY_X_OFFSET);
            let chimney_y = house_y.saturating_add(3);
            self.chimney.update(chimney_x, chimney_y, &mut rng);
        }
    }

    fn apply_preset(&mut self, preset: WeatherPreset) {
        if self.preset == preset {
            return;
        }
        self.preset = preset;
        match self.preset {
            WeatherPreset::Sunny => {
                self.rain.set_intensity(RainIntensity::Drizzle);
                self.rain.set_wind(0.0, 270.0);
                self.snow.set_intensity(SnowIntensity::Light);
                self.snow.set_wind(0.0, 270.0);
                self.fog.set_intensity(FogIntensity::Light);
            }
            WeatherPreset::Rainy => {
                self.rain.set_intensity(RainIntensity::Heavy);
                self.rain.set_wind(14.0, 248.0);
                self.snow.set_intensity(SnowIntensity::Light);
                self.snow.set_wind(0.0, 270.0);
                self.fog.set_intensity(FogIntensity::Light);
            }
            WeatherPreset::Snowy => {
                self.rain.set_intensity(RainIntensity::Drizzle);
                self.rain.set_wind(0.0, 270.0);
                self.snow.set_intensity(SnowIntensity::Heavy);
                self.snow.set_wind(3.0, 270.0);
                self.fog.set_intensity(FogIntensity::Light);
            }
            WeatherPreset::Foggy => {
                self.rain.set_intensity(RainIntensity::Drizzle);
                self.rain.set_wind(0.0, 270.0);
                self.snow.set_intensity(SnowIntensity::Light);
                self.snow.set_wind(0.0, 270.0);
                self.fog.set_intensity(FogIntensity::Medium);
            }
            WeatherPreset::Stormy => {
                self.rain.set_intensity(RainIntensity::Storm);
                self.rain.set_wind(18.0, 248.0);
                self.snow.set_intensity(SnowIntensity::Light);
                self.snow.set_wind(0.0, 270.0);
                self.fog.set_intensity(FogIntensity::Heavy);
            }
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, finish_sprint: bool) {
        self.renderer.clear();

        for pass in RENDER_PIPELINE {
            pass(self, self.quality, finish_sprint);
        }

        self.renderer.flush_to(area, buf);
    }

    fn render_sky_layer(&mut self, quality: RenderQuality, _finish_sprint: bool) {
        let _ = self.stars.render(&mut self.renderer);
        let _ = self.moon.render(&mut self.renderer);
        let _ = self.clouds.render(&mut self.renderer);

        if quality.show_airplanes && matches!(self.preset, WeatherPreset::Sunny) {
            let _ = self.airplanes.render(&mut self.renderer);
        }
        if quality.show_birds && !matches!(self.preset, WeatherPreset::Stormy) {
            let _ = self.birds.render(&mut self.renderer);
        }
    }

    fn render_mid_layer(&mut self, quality: RenderQuality, _finish_sprint: bool) {
        let horizon = self.renderer_height().saturating_sub(6);

        if quality.show_mountains {
            self.draw_mountains(horizon);
        }

        if quality.show_house {
            let house_x = self.renderer_width().saturating_sub(House::WIDTH).saturating_sub(2);
            let house_y = self
                .renderer_height()
                .saturating_sub(House::HEIGHT)
                .saturating_sub(2);
            let _ = self.house.render(&mut self.renderer, house_x, house_y, true);
            let _ = self.chimney.render(&mut self.renderer);
        }
    }

    fn render_ground_layer(&mut self, quality: RenderQuality, _finish_sprint: bool) {
        let horizon = self.renderer_height().saturating_sub(6);

        self.draw_road();
        self.draw_flora(horizon, quality.show_flowers);
        let _ = self.leaves.render(&mut self.renderer);

        if quality.show_fireflies {
            let _ = self.fireflies.render(&mut self.renderer);
        }
    }

    fn render_fx_layer(&mut self, quality: RenderQuality, finish_sprint: bool) {
        let width = self.renderer_width();
        let horizon = self.renderer_height().saturating_sub(6);

        if quality.show_snow && self.preset.is_snowy() {
            let _ = self.snow.render(&mut self.renderer);
        }
        if quality.show_fog && self.preset.is_foggy() {
            let _ = self.fog.render(&mut self.renderer);
        }
        if self.preset.is_rainy() {
            let _ = self.rain.render(&mut self.renderer);
        }

        if quality.show_thunderstorm && self.preset.has_thunder() {
            let _ = self.thunderstorm.render(&mut self.renderer);
            if self.thunderstorm.is_flashing() {
                let flash_y = horizon.saturating_sub(1);
                let _ = self.renderer.render_line_colored(
                    0,
                    flash_y,
                    &".".repeat(width as usize),
                    CtColor::White,
                );
            }
        }

        if finish_sprint {
            let flag_x = width.saturating_sub(9);
            let flag_y = horizon.saturating_sub(2);
            let _ = self
                .renderer
                .render_line_colored(flag_x, flag_y, "FINISH", CtColor::Yellow);
            let _ = self
                .renderer
                .render_char(flag_x.saturating_sub(1), flag_y + 1, '⚑', CtColor::DarkYellow);
        }
    }

    fn draw_mountains(&mut self, horizon: u16) {
        if horizon < 3 {
            return;
        }

        let mut x = 0u16;
        while x < self.renderer_width() {
            let pattern = ["   /\\    ", "  /  \\   ", " /_/\\_\\  "];
            for (idx, line) in pattern.iter().enumerate() {
                let y = horizon.saturating_sub(3).saturating_add(idx as u16);
                let _ = self
                    .renderer
                    .render_line_colored(x, y, line, CtColor::DarkGrey);
            }
            x = x.saturating_add(12);
        }
    }

    fn draw_flora(&mut self, horizon: u16, show_flowers: bool) {
        let width = self.renderer_width();
        if width < 4 {
            return;
        }

        let tree_line = horizon.saturating_add(1);
        let flowers = horizon.saturating_add(2);

        for x in (2..width.saturating_sub(2)).step_by(9) {
            let _ = self.renderer.render_char(x, tree_line, '♣', CtColor::Green);
            let _ = self
                .renderer
                .render_char(x.saturating_add(1), tree_line, '♠', CtColor::DarkGreen);
        }

        if show_flowers {
            for x in (1..width.saturating_sub(1)).step_by(5) {
                let flower = match x % 3 {
                    0 => '✿',
                    1 => '❀',
                    _ => '✽',
                };
                let _ = self.renderer.render_char(x, flowers, flower, CtColor::Magenta);
                let _ = self
                    .renderer
                    .render_char(x.saturating_add(1), flowers, '"', CtColor::Green);
            }
        }
    }

    fn draw_road(&mut self) {
        let width = self.renderer_width();
        let height = self.renderer_height();
        if width < 2 || height < 3 {
            return;
        }

        let road_y = height.saturating_sub(3);
        let _ = self
            .renderer
            .render_line_colored(0, road_y, &"=".repeat(width as usize), CtColor::DarkGrey);
        for x in (0..width).step_by(3) {
            let _ = self.renderer.render_char(x, road_y + 1, '-', CtColor::Yellow);
        }
        let _ = self.renderer.render_line_colored(
            0,
            height.saturating_sub(1),
            &"_".repeat(width as usize),
            CtColor::Green,
        );
    }

    fn renderer_width(&self) -> u16 {
        self.renderer.size().0
    }

    fn renderer_height(&self) -> u16 {
        self.renderer.size().1
    }
}

fn weather_state() -> &'static Mutex<WeatherScene> {
    static STATE: OnceLock<Mutex<WeatherScene>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(WeatherScene::new(100, 28)))
}

fn phase_seconds() -> f32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f32()
}

fn render_pomodoro_countdown(
    area: Rect,
    buf: &mut Buffer,
    text: &str,
    shift_x: i16,
    shift_y: i16,
) {
    let mut lines = super::figlet_lines(text, area.width as usize);
    if lines.is_empty() {
        lines.push(text.to_string());
    }
    let mut start_y = area.y as i16 + (area.height.saturating_sub(lines.len() as u16) / 2) as i16;
    start_y += shift_y;
    let shifted = Rect {
        x: area.x.saturating_add_signed(shift_x),
        y: area.y,
        width: area.width,
        height: area.height,
    };
    for (idx, line) in lines.iter().enumerate() {
        let y = start_y + idx as i16;
        if y < area.y as i16 || y >= area.bottom() as i16 {
            break;
        }
        super::put_centered(buf, shifted, y, line, ratatui::style::Color::Rgb(208, 42, 42));
    }
}
