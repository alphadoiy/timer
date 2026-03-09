use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use chrono::{Local, Timelike};
use serde::Deserialize;

const IPINFO_URL: &str = "https://ipinfo.io/json";
const OPEN_METEO_URL: &str = "https://api.open-meteo.com/v1/forecast";
const REFRESH_INTERVAL: Duration = Duration::from_secs(300);
static LOCATION_CONFIG: OnceLock<LocationConfig> = OnceLock::new();

#[derive(Clone, Copy)]
struct LocationConfig {
    coords: Option<(f64, f64)>,
    auto_location: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherCondition {
    Clear,
    PartlyCloudy,
    Overcast,
    Fog,
    Drizzle,
    Rain,
    FreezingRain,
    Snow,
    SnowGrains,
    RainShowers,
    SnowShowers,
    Thunderstorm,
    ThunderstormHail,
}

impl WeatherCondition {
    pub fn label(self) -> &'static str {
        match self {
            Self::Clear => "Clear",
            Self::PartlyCloudy => "PartlyCloudy",
            Self::Overcast => "Overcast",
            Self::Fog => "Fog",
            Self::Drizzle => "Drizzle",
            Self::Rain => "Rain",
            Self::FreezingRain => "FreezingRain",
            Self::Snow => "Snow",
            Self::SnowGrains => "SnowGrains",
            Self::RainShowers => "RainShowers",
            Self::SnowShowers => "SnowShowers",
            Self::Thunderstorm => "Thunderstorm",
            Self::ThunderstormHail => "ThunderstormHail",
        }
    }

    pub fn ui_text(self) -> &'static str {
        match self {
            Self::Clear => "Clear",
            Self::PartlyCloudy => "Partly Cloudy",
            Self::Overcast => "Overcast",
            Self::Fog => "Fog",
            Self::Drizzle => "Drizzle",
            Self::Rain => "Rain",
            Self::FreezingRain => "Freezing Rain",
            Self::Snow => "Snow",
            Self::SnowGrains => "Snow Grains",
            Self::RainShowers => "Rain Showers",
            Self::SnowShowers => "Snow Showers",
            Self::Thunderstorm => "Thunderstorm",
            Self::ThunderstormHail => "Thunderstorm with Hail",
        }
    }

    pub fn is_raining(self) -> bool {
        matches!(
            self,
            Self::Drizzle
                | Self::Rain
                | Self::RainShowers
                | Self::FreezingRain
                | Self::Thunderstorm
                | Self::ThunderstormHail
        )
    }

    pub fn is_snowing(self) -> bool {
        matches!(self, Self::Snow | Self::SnowGrains | Self::SnowShowers)
    }

    pub fn is_thunderstorm(self) -> bool {
        matches!(self, Self::Thunderstorm | Self::ThunderstormHail)
    }

    pub fn is_cloudy(self) -> bool {
        matches!(self, Self::PartlyCloudy | Self::Overcast)
    }

    pub fn is_foggy(self) -> bool {
        matches!(self, Self::Fog)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LiveWeather {
    pub condition: WeatherCondition,
    pub temperature_c: f32,
    pub wind_kmh: f32,
    pub precipitation_mm: f32,
    pub is_day: bool,
    pub attribution: &'static str,
}

impl Default for LiveWeather {
    fn default() -> Self {
        let hour = Local::now().hour();
        Self {
            condition: WeatherCondition::PartlyCloudy,
            temperature_c: 20.0,
            wind_kmh: 8.0,
            precipitation_mm: 0.0,
            is_day: (6..18).contains(&hour),
            attribution: "Awaiting weather data",
        }
    }
}

#[derive(Deserialize)]
struct IpInfoResponse {
    loc: String,
}

#[derive(Deserialize)]
struct OpenMeteoResponse {
    current: OpenMeteoCurrent,
}

#[derive(Deserialize)]
struct OpenMeteoCurrent {
    temperature_2m: f32,
    is_day: i32,
    precipitation: f32,
    weather_code: i32,
    wind_speed_10m: f32,
}

pub fn spawn_weather_worker(lat: Option<f64>, lon: Option<f64>) -> Receiver<LiveWeather> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let config = configured_location_config();
        let mut coords = config.coords.or(match (lat, lon) {
            (Some(lat), Some(lon)) => Some((lat, lon)),
            _ => None,
        });
        if coords.is_none() && config.auto_location {
            coords = detect_location().ok();
        }

        let mut fallback = LiveWeather::default();

        loop {
            if coords.is_none() && config.auto_location {
                coords = detect_location().ok();
            }

            if let Some((latitude, longitude)) = coords {
                if let Ok(weather) = fetch_weather(latitude, longitude) {
                    fallback = weather;
                    if tx.send(weather).is_err() {
                        break;
                    }
                } else if tx.send(fallback).is_err() {
                    break;
                }
            } else if tx.send(fallback).is_err() {
                break;
            }

            thread::sleep(REFRESH_INTERVAL);
        }
    });
    rx
}

pub fn configured_coords() -> Option<(f64, f64)> {
    configured_location_config().coords
}

pub fn configure_location(coords: Option<(f64, f64)>, auto_location: bool) {
    let _ = LOCATION_CONFIG.set(LocationConfig {
        coords,
        auto_location,
    });
}

fn configured_location_config() -> LocationConfig {
    LOCATION_CONFIG.get().copied().unwrap_or(LocationConfig {
        coords: None,
        auto_location: true,
    })
}

fn detect_location() -> anyhow::Result<(f64, f64)> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()?;
    let resp = client.get(IPINFO_URL).send()?.error_for_status()?;
    let payload: IpInfoResponse = resp.json()?;
    let mut parts = payload.loc.split(',');
    let lat = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("ipinfo location missing latitude"))?
        .parse::<f64>()?;
    let lon = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("ipinfo location missing longitude"))?
        .parse::<f64>()?;
    Ok((lat, lon))
}

fn fetch_weather(lat: f64, lon: f64) -> anyhow::Result<LiveWeather> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let url = format!(
        "{OPEN_METEO_URL}?latitude={lat}&longitude={lon}&current=temperature_2m,is_day,precipitation,weather_code,wind_speed_10m&temperature_unit=celsius&wind_speed_unit=kmh&precipitation_unit=mm&timezone=auto"
    );
    let resp = client.get(url).send()?.error_for_status()?;
    let payload: OpenMeteoResponse = resp.json()?;
    let current = payload.current;
    Ok(LiveWeather {
        condition: map_wmo_to_condition(current.weather_code),
        temperature_c: current.temperature_2m,
        wind_kmh: current.wind_speed_10m,
        precipitation_mm: if current.precipitation.is_finite() {
            current.precipitation.max(0.0)
        } else {
            0.0
        },
        is_day: current.is_day == 1,
        attribution: "Weather data: Open-Meteo.com",
    })
}

pub fn map_wmo_to_condition(code: i32) -> WeatherCondition {
    match code {
        0 => WeatherCondition::Clear,
        1 | 2 => WeatherCondition::PartlyCloudy,
        3 => WeatherCondition::Overcast,
        45 | 48 => WeatherCondition::Fog,
        51 | 53 | 55 => WeatherCondition::Drizzle,
        56 | 57 => WeatherCondition::FreezingRain,
        61 | 63 | 65 => WeatherCondition::Rain,
        66 | 67 => WeatherCondition::FreezingRain,
        71 | 73 | 75 => WeatherCondition::Snow,
        77 => WeatherCondition::SnowGrains,
        80..=82 => WeatherCondition::RainShowers,
        85 | 86 => WeatherCondition::SnowShowers,
        95 => WeatherCondition::Thunderstorm,
        96 | 99 => WeatherCondition::ThunderstormHail,
        _ => WeatherCondition::Clear,
    }
}

#[cfg(test)]
mod tests {
    use super::{WeatherCondition, map_wmo_to_condition};

    #[test]
    fn maps_wmo_codes_to_upstream_presets() {
        assert_eq!(map_wmo_to_condition(0), WeatherCondition::Clear);
        assert_eq!(map_wmo_to_condition(1), WeatherCondition::PartlyCloudy);
        assert_eq!(map_wmo_to_condition(3), WeatherCondition::Overcast);
        assert_eq!(map_wmo_to_condition(45), WeatherCondition::Fog);
        assert_eq!(map_wmo_to_condition(51), WeatherCondition::Drizzle);
        assert_eq!(map_wmo_to_condition(56), WeatherCondition::FreezingRain);
        assert_eq!(map_wmo_to_condition(61), WeatherCondition::Rain);
        assert_eq!(map_wmo_to_condition(71), WeatherCondition::Snow);
        assert_eq!(map_wmo_to_condition(77), WeatherCondition::SnowGrains);
        assert_eq!(map_wmo_to_condition(80), WeatherCondition::RainShowers);
        assert_eq!(map_wmo_to_condition(85), WeatherCondition::SnowShowers);
        assert_eq!(map_wmo_to_condition(95), WeatherCondition::Thunderstorm);
        assert_eq!(map_wmo_to_condition(96), WeatherCondition::ThunderstormHail);
    }
}
