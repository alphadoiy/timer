use std::time::{Duration, Instant};

use ratatui::{buffer::Buffer, layout::Rect};

use crate::{
    render::weathr::{
        BrailleWeatherCanvas,
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

const SIM_STEP: Duration = Duration::from_millis(33);
const MAX_SIM_STEPS_PER_FRAME: u8 = 8;
const SUNNY_FRAME_DELAY: Duration = Duration::from_millis(500);

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
    canvas: BrailleWeatherCanvas,
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
            canvas: BrailleWeatherCanvas::new(area),
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
            self.clouds.update(w, h, is_clear, &mut rng);
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
            self.chimney.update(chimney_x, house_y, &mut rng);
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

        if !flags.is_day {
            self.stars.render_braille(&mut self.canvas, dark_bg);
            self.moon.render_braille(&mut self.canvas, dark_bg);
            if self.should_show_fireflies() {
                self.fireflies.render_braille(&mut self.canvas, dark_bg);
            }
        }

        if self.should_show_sun()
            && !flags.is_raining
            && !flags.is_thunderstorm
            && !flags.is_snowing
        {
            self.sunny_animation.render_braille(
                &mut self.canvas,
                self.animation_controller.current_frame(),
                dark_bg,
            );
        }

        self.clouds.render_braille(&mut self.canvas, dark_bg);

        if !flags.is_raining && !flags.is_thunderstorm && !flags.is_snowing && flags.is_day {
            self.birds.render_braille(&mut self.canvas, dark_bg);
            if self.should_show_butterflies() {
                self.butterflies.render_braille(&mut self.canvas, dark_bg);
            }
        }

        if !flags.is_raining && !flags.is_thunderstorm && !flags.is_snowing && !flags.is_foggy {
            self.airplanes.render_braille(&mut self.canvas, dark_bg);
        }

        let ground_weather = GroundWeather {
            is_raining: flags.is_raining,
            is_snowing: flags.is_snowing,
            is_thunderstorm: flags.is_thunderstorm,
        };
        self.scene
            .render_braille(&mut self.canvas, flags.is_day, ground_weather, dark_bg);

        if !flags.is_raining && !flags.is_thunderstorm {
            self.chimney.render_braille(&mut self.canvas, dark_bg);
        }

        if flags.is_thunderstorm {
            self.rain.render_braille(&mut self.canvas, dark_bg);
            self.thunderstorm.render_braille(&mut self.canvas, dark_bg);
        } else if flags.is_raining {
            self.rain.render_braille(&mut self.canvas, dark_bg);
        } else if flags.is_snowing {
            self.snow.render_braille(&mut self.canvas, dark_bg);
        }

        if flags.is_foggy {
            self.fog.render_braille(&mut self.canvas, dark_bg);
        }

        if self.should_show_leaves() {
            self.leaves.render_braille(&mut self.canvas, dark_bg);
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
        self.canvas.put_text(attr_x, attr_y, attribution, attr_color);
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

    fn should_show_leaves(&self) -> bool {
        let c = self.current_weather.condition;
        if c.is_raining() || c.is_thunderstorm() || c.is_snowing() {
            return false;
        }
        (5.0..=22.0).contains(&self.current_weather.temperature_c)
    }
}

fn format_precip_mm(value: f32) -> String {
    if !value.is_finite() || value <= 0.05 {
        "0".to_string()
    } else {
        format!("{value:.1}")
    }
}
