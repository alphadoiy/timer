use std::{
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crossterm::style::Color as CtColor;
use unicode_width::UnicodeWidthStr;

use crate::{
    render::weathr::{
        TerminalRenderer, ViewportTransform,
        animation::{
            AnimationController, airplanes::AirplaneSystem, birds::BirdSystem,
            butterflies::ButterflySystem,
            chimney::ChimneySmoke, clouds::CloudSystem, fireflies::FireflySystem, fog::FogSystem,
            leaves::FallingLeaves, moon::MoonSystem, raindrops::RaindropSystem,
            snow::SnowSystem,
            stars::StarSystem, sunny::SunnyAnimation, thunderstorm::ThunderstormSystem,
        },
        scene::{WorldScene, house::House, ground::GroundWeather},
        types::{FogIntensity, RainIntensity, SnowIntensity},
    },
    weather_live::{LiveWeather, WeatherCondition, configured_coords, spawn_weather_worker},
};

use super::*;

const SIM_STEP: Duration = Duration::from_millis(33);
const MAX_SIM_STEPS_PER_FRAME: u8 = 8;
const LOGICAL_WIDTH: u16 = 120;
const LOGICAL_HEIGHT: u16 = 34;
const SUNNY_FRAME_DELAY: Duration = Duration::from_millis(500);

pub(crate) fn current_weather_summary() -> String {
    let state = weather_state();
    let state = state.lock().expect("weather animation mutex poisoned");
    let weather = state.current_weather;
    let precip = format_precip_mm(weather.precipitation_mm);
    format!(
        "{}  {:.1}C  {:.1}km/h  {}mm",
        weather.condition.label(),
        weather.temperature_c,
        weather.wind_kmh,
        precip
    )
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
    render_pomodoro_countdown(rows[0], buf, &countdown, jx, jy, theme);

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
        .constraints([
            Constraint::Length(1),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(inner);
    let top = lanes[0];
    let world = lanes[1];
    let bottom = lanes[2];

    super::put_text(buf, top.x as i16, top.y as i16, "25:00", theme.text);
    let right_x = top.right() as i16 - UnicodeWidthStr::width("00:00") as i16;
    super::put_text(buf, right_x, top.y as i16, "00:00", theme.text);
    super::put_text(
        buf,
        (right_x - 6).max(top.x as i16),
        top.y as i16,
        "FIN",
        theme.highlight,
    );

    render_weathr_animation(world, buf, finish_sprint);

    Paragraph::new(Line::from(Span::styled(
        "Focus: upstream weathr presets",
        Style::default().fg(theme.subtext),
    )))
    .alignment(Alignment::Center)
    .render(bottom, buf);
}

fn render_weathr_animation(area: Rect, buf: &mut Buffer, finish_sprint: bool) {
    let state = weather_state();
    let mut state = state.lock().expect("weather animation mutex poisoned");
    state.update(area.width, area.height);
    state.render(area, buf, finish_sprint);
}

#[derive(Clone, Copy)]
struct WeatherFlags {
    is_raining: bool,
    is_snowing: bool,
    is_thunderstorm: bool,
    is_cloudy: bool,
    is_foggy: bool,
    is_day: bool,
}

impl WeatherFlags {
    fn from_weather(weather: LiveWeather) -> Self {
        let c = weather.condition;
        Self {
            is_raining: c.is_raining() && !c.is_thunderstorm(),
            is_snowing: c.is_snowing(),
            is_thunderstorm: c.is_thunderstorm(),
            is_cloudy: c.is_cloudy(),
            is_foggy: c.is_foggy(),
            is_day: weather.is_day,
        }
    }
}

struct WeatherScene {
    renderer: TerminalRenderer,
    scene: WorldScene,
    rain: RaindropSystem,
    snow: SnowSystem,
    fog: FogSystem,
    thunderstorm: ThunderstormSystem,
    clouds: CloudSystem,
    birds: BirdSystem,
    airplanes: AirplaneSystem,
    stars: StarSystem,
    moon: MoonSystem,
    chimney: ChimneySmoke,
    fireflies: FireflySystem,
    butterflies: ButterflySystem,
    leaves: FallingLeaves,
    sunny_animation: SunnyAnimation,
    animation_controller: AnimationController,
    last_sunny_frame_time: Instant,
    current_weather: LiveWeather,
    weather_rx: std::sync::mpsc::Receiver<LiveWeather>,
    last_update: Instant,
    accumulator: Duration,
}

impl WeatherScene {
    fn new(width: u16, height: u16) -> Self {
        Self {
            renderer: TerminalRenderer::new(width, height),
            scene: WorldScene::new(width, height),
            rain: RaindropSystem::new(width, height, RainIntensity::Light),
            snow: SnowSystem::new(width, height, SnowIntensity::Light),
            fog: FogSystem::new(width, height, FogIntensity::Light),
            thunderstorm: ThunderstormSystem::new(width, height),
            clouds: CloudSystem::new(width, height),
            birds: BirdSystem::new(width, height),
            airplanes: AirplaneSystem::new(width, height),
            stars: StarSystem::new(width, height),
            moon: MoonSystem::new(width, height, Some(0.5)),
            chimney: ChimneySmoke::new(),
            fireflies: FireflySystem::new(width, height),
            butterflies: ButterflySystem::new(width, height),
            leaves: FallingLeaves::new(width, height),
            sunny_animation: SunnyAnimation::new(),
            animation_controller: AnimationController::new(),
            last_sunny_frame_time: Instant::now(),
            current_weather: LiveWeather::default(),
            weather_rx: {
                let configured = configured_coords();
                let (lat, lon) = configured
                    .map(|(lat, lon)| (Some(lat), Some(lon)))
                    .unwrap_or((None, None));
                spawn_weather_worker(lat, lon)
            },
            last_update: Instant::now(),
            accumulator: Duration::ZERO,
        }
    }

    fn update(&mut self, area_width: u16, area_height: u16) {
        self.update_cloud_safe_area(area_width, area_height);

        while let Ok(weather) = self.weather_rx.try_recv() {
            self.current_weather = weather;
            self.apply_weather_profile(weather);
            self.moon.set_phase(0.5);
        }

        let now = Instant::now();
        let delta = now
            .saturating_duration_since(self.last_update)
            .min(Duration::from_millis(250));
        self.last_update = now;
        self.accumulator = self.accumulator.saturating_add(delta);

        let mut steps = 0u8;
        while self.accumulator >= SIM_STEP && steps < MAX_SIM_STEPS_PER_FRAME {
            self.advance_simulation();
            self.accumulator = self.accumulator.saturating_sub(SIM_STEP);
            steps = steps.saturating_add(1);
        }

        let flags = WeatherFlags::from_weather(self.current_weather);
        if !flags.is_raining
            && !flags.is_thunderstorm
            && !flags.is_snowing
            && self.last_sunny_frame_time.elapsed() >= SUNNY_FRAME_DELAY
        {
            self.animation_controller.next_frame(&self.sunny_animation);
            self.last_sunny_frame_time = Instant::now();
        }
    }

    fn apply_weather_profile(&mut self, weather: LiveWeather) {
        let c = weather.condition;
        self.rain.set_intensity(match c {
            WeatherCondition::Drizzle => RainIntensity::Drizzle,
            WeatherCondition::Rain | WeatherCondition::RainShowers => RainIntensity::Light,
            WeatherCondition::FreezingRain | WeatherCondition::Thunderstorm => RainIntensity::Heavy,
            WeatherCondition::ThunderstormHail => RainIntensity::Storm,
            _ => RainIntensity::Light,
        });
        self.snow.set_intensity(match c {
            WeatherCondition::SnowGrains => SnowIntensity::Light,
            WeatherCondition::SnowShowers => SnowIntensity::Medium,
            WeatherCondition::Snow => SnowIntensity::Heavy,
            _ => SnowIntensity::Light,
        });
        self.fog.set_intensity(match c {
            WeatherCondition::Fog => FogIntensity::Medium,
            _ => FogIntensity::Light,
        });

        let wind = weather.wind_kmh.clamp(0.0, 45.0);
        let dir = if c.is_thunderstorm() { 225.0 } else { 255.0 };
        self.rain.set_wind(wind, dir);
        self.snow.set_wind((wind * 0.7).max(1.0), dir);
        self.clouds.set_wind((wind * 0.8).max(4.0), dir);
    }

    fn advance_simulation(&mut self) {
        let mut rng = rand::rng();
        let (w, h) = self.renderer.size();
        let flags = WeatherFlags::from_weather(self.current_weather);

        if !flags.is_day {
            self.stars.update(w, h, &mut rng);
            self.moon.update(w, h);
            if self.should_show_fireflies() {
                let horizon = h.saturating_sub(WorldScene::GROUND_HEIGHT);
                self.fireflies.update(w, h, horizon, &mut rng);
            }
        }

        if !flags.is_raining && !flags.is_thunderstorm && !flags.is_snowing && flags.is_day {
            self.birds.update(w, h, &mut rng);
            if self.should_show_butterflies() {
                let horizon = h.saturating_sub(WorldScene::GROUND_HEIGHT);
                self.butterflies.update(w, h, horizon, &mut rng);
            }
        }

        if flags.is_cloudy || self.current_weather.condition == WeatherCondition::Clear {
            let is_clear = self.current_weather.condition == WeatherCondition::Clear;
            let cloud_color = if is_clear {
                CtColor::White
            } else if self.current_weather.condition == WeatherCondition::PartlyCloudy {
                CtColor::Grey
            } else {
                CtColor::DarkGrey
            };
            self.clouds.set_cloud_color(is_clear);
            self.clouds.update(w, h, is_clear, cloud_color, &mut rng);
        }

        if !flags.is_raining && !flags.is_thunderstorm && !flags.is_snowing && !flags.is_foggy {
            self.airplanes.update(w, h, &mut rng);
        }

        if flags.is_thunderstorm {
            self.rain.update(w, h, &mut rng);
            self.thunderstorm.update(w, h, &mut rng);
        } else if flags.is_raining {
            self.rain.update(w, h, &mut rng);
        } else if flags.is_snowing {
            self.snow.update(w, h, &mut rng);
        }

        if flags.is_foggy {
            self.fog.update(w, h, &mut rng);
        }

        if self.should_show_leaves() {
            self.leaves.update(w, h, &mut rng);
        }

        if !flags.is_raining && !flags.is_thunderstorm {
            let horizon = h.saturating_sub(WorldScene::GROUND_HEIGHT);
            let house_x = (w / 2).saturating_sub(House::WIDTH / 2);
            let house_y = horizon.saturating_sub(House::HEIGHT);
            let chimney_x = house_x + House::CHIMNEY_X_OFFSET;
            let chimney_y = house_y;
            self.chimney.update(chimney_x, chimney_y, &mut rng);
        }
    }

    fn update_cloud_safe_area(&mut self, area_width: u16, area_height: u16) {
        let (w, h) = self.renderer.size();
        let viewport =
            ViewportTransform::cover(w, h, Rect::new(0, 0, area_width.max(1), area_height.max(1)));

        let visible_w = area_width as f32 / viewport.scale;
        let visible_h = area_height as f32 / viewport.scale;
        let left = ((w as f32 - visible_w) * 0.5).max(0.0);
        let top = ((h as f32 - visible_h) * 0.5).max(0.0);
        let right = (left + visible_w).min(w as f32);
        let bottom = (top + visible_h).min(h as f32);

        let margin_x = 2.0;
        let margin_y = 1.0;
        self.clouds.set_safe_area(
            (left + margin_x).min(right),
            (right - margin_x).max(left),
            (top + margin_y).min(bottom),
            (bottom - margin_y).max(top),
        );
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, finish_sprint: bool) {
        self.renderer.clear();
        let (w, h) = self.renderer.size();
        self.scene.update_size(w, h);

        let flags = WeatherFlags::from_weather(self.current_weather);

        if !flags.is_day {
            let _ = self.stars.render(&mut self.renderer);
            let _ = self.moon.render(&mut self.renderer);
            if self.should_show_fireflies() {
                let _ = self.fireflies.render(&mut self.renderer);
            }
        }

        if self.should_show_sun()
            && !flags.is_raining
            && !flags.is_thunderstorm
            && !flags.is_snowing
        {
            let animation_y = if h > 20 { 3 } else { 2 };
            let _ = self.sunny_animation.render_colored(
                &mut self.renderer,
                self.animation_controller.current_frame(),
                animation_y,
            );
        }

        let _ = self.clouds.render(&mut self.renderer);

        if !flags.is_raining && !flags.is_thunderstorm && !flags.is_snowing && flags.is_day {
            let _ = self.birds.render(&mut self.renderer);
            if self.should_show_butterflies() {
                let _ = self.butterflies.render(&mut self.renderer);
            }
        }

        if !flags.is_raining && !flags.is_thunderstorm && !flags.is_snowing && !flags.is_foggy {
            let _ = self.airplanes.render(&mut self.renderer);
        }

        let ground_weather = GroundWeather {
            is_raining: flags.is_raining,
            is_snowing: flags.is_snowing,
            is_thunderstorm: flags.is_thunderstorm,
        };
        let _ = self.scene.render_with_weather(&mut self.renderer, flags.is_day, ground_weather);

        if !flags.is_raining && !flags.is_thunderstorm {
            let _ = self.chimney.render(&mut self.renderer);
        }

        if flags.is_thunderstorm {
            let _ = self.rain.render(&mut self.renderer);
            let _ = self.thunderstorm.render(&mut self.renderer);
        } else if flags.is_raining {
            let _ = self.rain.render(&mut self.renderer);
        } else if flags.is_snowing {
            let _ = self.snow.render(&mut self.renderer);
        }

        if flags.is_foggy {
            let _ = self.fog.render(&mut self.renderer);
        }

        if self.should_show_leaves() {
            let _ = self.leaves.render(&mut self.renderer);
        }

        if finish_sprint {
            let flag_x = w.saturating_sub(9);
            let flag_y = h.saturating_sub(4);
            let _ = self
                .renderer
                .render_line_colored(flag_x, flag_y, "FINISH", CtColor::Yellow);
            let _ = self.renderer.render_char(
                flag_x.saturating_sub(1),
                flag_y + 1,
                '⚑',
                CtColor::DarkYellow,
            );
        }

        let hud = self.weather_hud_line();
        let _ = self.renderer.render_line_colored(2, 1, &hud, CtColor::Cyan);

        let attribution = self.current_weather.attribution;
        let attr_x = w.saturating_sub(attribution.len() as u16).saturating_sub(2);
        let attr_y = h.saturating_sub(1);
        let _ = self
            .renderer
            .render_line_colored(attr_x, attr_y, attribution, CtColor::DarkGrey);

        let viewport =
            ViewportTransform::cover(self.renderer.size().0, self.renderer.size().1, area);
        self.renderer.flush_cover_to(area, buf, viewport);
    }

    fn weather_hud_line(&self) -> String {
        let precip = format_precip_mm(self.current_weather.precipitation_mm);
        format!(
            "Weather: {} | Temp: {:.1}C | Wind: {:.1}km/h | Precip: {}mm | Press 'q' to quit",
            self.current_weather.condition.ui_text(),
            self.current_weather.temperature_c,
            self.current_weather.wind_kmh,
            precip
        )
    }

    fn should_show_sun(&self) -> bool {
        if !self.current_weather.is_day {
            return false;
        }
        matches!(
            self.current_weather.condition,
            WeatherCondition::Clear | WeatherCondition::PartlyCloudy
        )
    }

    fn should_show_fireflies(&self) -> bool {
        if self.current_weather.is_day {
            return false;
        }
        let is_warm = self.current_weather.temperature_c > 15.0;
        let clear_night = matches!(
            self.current_weather.condition,
            WeatherCondition::Clear | WeatherCondition::PartlyCloudy
        );
        let c = self.current_weather.condition;
        is_warm && clear_night && !c.is_raining() && !c.is_thunderstorm() && !c.is_snowing()
    }

    fn should_show_butterflies(&self) -> bool {
        if !self.current_weather.is_day {
            return false;
        }
        let temp = self.current_weather.temperature_c;
        let warm_enough = temp > 18.0;
        let c = self.current_weather.condition;
        warm_enough
            && matches!(c, WeatherCondition::Clear | WeatherCondition::PartlyCloudy)
            && !c.is_raining()
            && !c.is_snowing()
    }

    fn should_show_leaves(&self) -> bool {
        let c = self.current_weather.condition;
        if c.is_raining() || c.is_thunderstorm() || c.is_snowing() {
            return false;
        }
        let temp = self.current_weather.temperature_c;
        (5.0..=22.0).contains(&temp)
    }
}

fn weather_state() -> &'static Mutex<WeatherScene> {
    static STATE: OnceLock<Mutex<WeatherScene>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(WeatherScene::new(LOGICAL_WIDTH, LOGICAL_HEIGHT)))
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
    theme: Theme,
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
        let solid_line = super::solidify_figlet_line(line);
        super::put_centered_gradient(
            buf,
            shifted,
            y,
            &solid_line,
            theme.highlight,
            theme.accent,
            theme.danger,
        );
    }
}

fn format_precip_mm(value: f32) -> String {
    if !value.is_finite() || value <= 0.05 {
        "0".to_string()
    } else {
        format!("{value:.1}")
    }
}
