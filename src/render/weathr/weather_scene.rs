use std::time::{Duration, Instant};

use chrono::{Local, Timelike};
use ratatui::{buffer::Buffer, layout::Rect};

use crate::{
    render::weathr::{
        HalfBlockCanvas,
        animation::{
            AnimationController, airplanes::AirplaneSystem, birds::BirdSystem,
            butterflies::ButterflySystem, chimney::ChimneySmoke, clouds::CloudSystem,
            fireflies::FireflySystem, fog::FogSystem, moon::MoonSystem, raindrops::RaindropSystem,
            snow::SnowSystem, stars::StarSystem, sunny::SunnyAnimation,
            thunderstorm::ThunderstormSystem,
        },
        scene::{WorldScene, ground::GroundWeather, house::House},
        types::{FogIntensity, RainIntensity, SnowIntensity},
    },
    weather_live::{LiveWeather, WeatherCondition, configured_coords, spawn_weather_worker},
};

const SIM_STEP: Duration = Duration::from_millis(33);
const MAX_SIM_STEPS_PER_FRAME: u8 = 8;
const SUNNY_FRAME_DELAY: Duration = Duration::from_millis(500);
const SUNRISE_HOUR: f32 = 6.0;
const SUNSET_HOUR: f32 = 18.0;
const CELESTIAL_MARGIN_X: f32 = 4.0;
const CELESTIAL_TOP_Y: f32 = 2.5;
const CELESTIAL_BOTTOM_PADDING: f32 = 2.5;
const SUN_RADIUS: f32 = 2.5;
const MOON_RADIUS: f32 = 3.5;

#[derive(Clone, Copy)]
struct CelestialPosition {
    x: f32,
    y: f32,
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

pub(crate) struct WeatherScene {
    canvas: HalfBlockCanvas,
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
    sunny_animation: SunnyAnimation,
    animation_controller: AnimationController,
    last_sunny_frame_time: Instant,
    pub(crate) current_weather: LiveWeather,
    weather_rx: std::sync::mpsc::Receiver<LiveWeather>,
    last_update: Instant,
    accumulator: Duration,
    last_w: u16,
    last_h: u16,
}

impl WeatherScene {
    pub(crate) fn new(width: u16, height: u16) -> Self {
        let area = Rect::new(0, 0, width, height);
        Self {
            canvas: HalfBlockCanvas::new(area),
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
            last_w: width,
            last_h: height,
        }
    }

    pub(crate) fn update(&mut self, area: Rect) {
        let w = area.width;
        let h = area.height;
        if w != self.last_w || h != self.last_h {
            self.last_w = w;
            self.last_h = h;
        }

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
            self.advance_simulation(w, h);
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

    fn advance_simulation(&mut self, w: u16, h: u16) {
        let mut rng = rand::rng();
        let flags = WeatherFlags::from_weather(self.current_weather);

        if !flags.is_day {
            self.stars.update(w, h, &mut rng);
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

        if flags.is_cloudy
            || flags.is_raining
            || flags.is_thunderstorm
            || flags.is_snowing
            || self.current_weather.condition == WeatherCondition::Clear
        {
            let is_clear = self.current_weather.condition == WeatherCondition::Clear;
            let use_dark_palette =
                matches!(self.current_weather.condition, WeatherCondition::Overcast)
                    || flags.is_raining
                    || flags.is_thunderstorm
                    || flags.is_snowing;
            self.clouds
                .update(w, h, is_clear, use_dark_palette, &mut rng);
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

        if !flags.is_raining && !flags.is_thunderstorm {
            let horizon = h.saturating_sub(WorldScene::GROUND_HEIGHT);
            let house_x = (w / 2).saturating_sub(House::WIDTH / 2);
            let house_y = horizon.saturating_sub(House::HEIGHT);
            let (chimney_x, chimney_y) = self.house_chimney_source(house_x, house_y);
            self.chimney.update(chimney_x, chimney_y, &mut rng);
        }
    }

    pub(crate) fn render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        finish_sprint: bool,
        dark_bg: bool,
    ) {
        self.canvas.resize(area);
        self.canvas.clear();
        let w = area.width;
        let h = area.height;
        self.scene.update_size(w, h);

        let flags = WeatherFlags::from_weather(self.current_weather);
        let sun_position = self.sun_position(w, h);
        let moon_position = self.moon_position(w, h);
        self.moon.set_position(
            moon_position.x.round() as u16,
            moon_position.y.round() as u16,
        );

        if !flags.is_day {
            self.stars.render(&mut self.canvas, dark_bg);
            if self.should_show_fireflies() {
                self.fireflies.render(&mut self.canvas, dark_bg);
            }
        }

        if self.should_show_sun()
            && !flags.is_raining
            && !flags.is_thunderstorm
            && !flags.is_snowing
        {
            self.sunny_animation.render_at(
                &mut self.canvas,
                self.animation_controller.current_frame(),
                dark_bg,
                sun_position.x,
                sun_position.y,
            );
        }

        self.clouds.render(&mut self.canvas, dark_bg);

        if !flags.is_raining && !flags.is_thunderstorm && !flags.is_snowing && flags.is_day {
            self.birds.render(&mut self.canvas, dark_bg);
            if self.should_show_butterflies() {
                self.butterflies.render(&mut self.canvas, dark_bg);
            }
        }

        if !flags.is_raining && !flags.is_thunderstorm && !flags.is_snowing && !flags.is_foggy {
            self.airplanes.render(&mut self.canvas, dark_bg);
        }

        let ground_weather = GroundWeather {
            is_raining: flags.is_raining,
            is_snowing: flags.is_snowing,
            is_thunderstorm: flags.is_thunderstorm,
        };
        self.scene
            .render(&mut self.canvas, flags.is_day, ground_weather, dark_bg);

        if !flags.is_day {
            self.moon.render(&mut self.canvas, dark_bg);
        }

        if !flags.is_raining && !flags.is_thunderstorm {
            self.chimney.render(&mut self.canvas, dark_bg);
        }

        if flags.is_thunderstorm {
            self.rain.render(&mut self.canvas, dark_bg);
            self.thunderstorm.render(&mut self.canvas, dark_bg);
        } else if flags.is_raining {
            self.rain.render(&mut self.canvas, dark_bg);
        } else if flags.is_snowing {
            self.snow.render(&mut self.canvas, dark_bg);
        }

        if flags.is_foggy {
            self.fog.render(&mut self.canvas, dark_bg);
        }

        self.render_hud(w, h, finish_sprint, dark_bg);
        self.canvas.flush(buf);
    }

    fn render_hud(&mut self, w: u16, h: u16, finish_sprint: bool, dark_bg: bool) {
        if finish_sprint {
            let flag_x = w.saturating_sub(9);
            let flag_y = h.saturating_sub(4);
            let color = if dark_bg {
                ratatui::style::Color::Yellow
            } else {
                ratatui::style::Color::Rgb(180, 120, 0)
            };
            self.canvas.put_text(flag_x, flag_y, "FINISH", color);
            self.canvas
                .put_text(flag_x.saturating_sub(1), flag_y + 1, "⚑", color);
        }

        let hud = self.weather_hud_line();
        let hud_color = if dark_bg {
            ratatui::style::Color::Cyan
        } else {
            ratatui::style::Color::Rgb(0, 100, 140)
        };
        self.canvas.put_text(2, 1, &hud, hud_color);

        let attribution = self.current_weather.attribution;
        let attr_x = w.saturating_sub(attribution.len() as u16).saturating_sub(2);
        let attr_y = h.saturating_sub(1);
        let attr_color = if dark_bg {
            ratatui::style::Color::DarkGray
        } else {
            ratatui::style::Color::Gray
        };
        self.canvas
            .put_text(attr_x, attr_y, attribution, attr_color);
    }

    fn weather_hud_line(&self) -> String {
        let precip = format_precip_mm(self.current_weather.precipitation_mm);
        format!(
            "Weather: {} | Temp: {:.1}C | Wind: {:.1}km/h | Precip: {}mm",
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
        let c = self.current_weather.condition;
        self.current_weather.temperature_c > 15.0
            && matches!(c, WeatherCondition::Clear | WeatherCondition::PartlyCloudy)
            && !c.is_raining()
            && !c.is_thunderstorm()
            && !c.is_snowing()
    }

    fn should_show_butterflies(&self) -> bool {
        if !self.current_weather.is_day {
            return false;
        }
        let c = self.current_weather.condition;
        self.current_weather.temperature_c > 18.0
            && matches!(c, WeatherCondition::Clear | WeatherCondition::PartlyCloudy)
            && !c.is_raining()
            && !c.is_snowing()
    }

    fn house_chimney_source(&self, house_x: u16, house_y: u16) -> (u16, u16) {
        House.chimney_smoke_source(house_x, house_y)
    }

    fn sun_position(&self, w: u16, h: u16) -> CelestialPosition {
        let time_hours = local_time_hours();
        let progress = ((time_hours - SUNRISE_HOUR) / (SUNSET_HOUR - SUNRISE_HOUR)).clamp(0.0, 1.0);
        celestial_position(progress, w, h, SUN_RADIUS)
    }

    fn moon_position(&self, w: u16, h: u16) -> CelestialPosition {
        let time_hours = local_time_hours();
        let progress = if time_hours >= SUNSET_HOUR {
            (time_hours - SUNSET_HOUR) / (24.0 - SUNSET_HOUR + SUNRISE_HOUR)
        } else {
            (time_hours + (24.0 - SUNSET_HOUR)) / (24.0 - SUNSET_HOUR + SUNRISE_HOUR)
        }
        .clamp(0.0, 1.0);
        celestial_position(progress, w, h, MOON_RADIUS)
    }
}

fn local_time_hours() -> f32 {
    let now = Local::now();
    now.hour() as f32 + now.minute() as f32 / 60.0 + now.second() as f32 / 3600.0
}

fn celestial_position(progress: f32, w: u16, h: u16, radius: f32) -> CelestialPosition {
    let horizon_y = h.saturating_sub(WorldScene::GROUND_HEIGHT) as f32;
    let skyline_top = (horizon_y - 13.5).max(CELESTIAL_TOP_Y + radius);
    let floor_y = (skyline_top - radius - CELESTIAL_BOTTOM_PADDING).max(CELESTIAL_TOP_Y + radius);
    let usable_w = (w as f32 - CELESTIAL_MARGIN_X * 2.0).max(8.0);
    let x = CELESTIAL_MARGIN_X + progress * usable_w;
    let arc = 1.0 - (2.0 * progress - 1.0).powi(2);
    let y = floor_y - arc * (floor_y - (CELESTIAL_TOP_Y + radius));
    CelestialPosition { x, y }
}

fn format_precip_mm(value: f32) -> String {
    if !value.is_finite() || value <= 0.05 {
        "0".to_string()
    } else {
        format!("{value:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::{CELESTIAL_TOP_Y, MOON_RADIUS, SUN_RADIUS, celestial_position};

    #[test]
    fn celestial_path_stays_above_skyline() {
        let h = 34;
        for progress in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
            let sun = celestial_position(progress, 120, h, SUN_RADIUS);
            let moon = celestial_position(progress, 120, h, MOON_RADIUS);
            assert!(sun.y >= CELESTIAL_TOP_Y + SUN_RADIUS - 0.1);
            assert!(moon.y >= CELESTIAL_TOP_Y + MOON_RADIUS - 0.1);
            assert!(sun.y <= h as f32 - 20.0);
            assert!(moon.y <= h as f32 - 20.0);
        }
    }
}
