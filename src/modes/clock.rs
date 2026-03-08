use chrono::{Local, Timelike};

#[derive(Debug, Default)]
pub struct ClockMode;

#[derive(Debug, Clone)]
pub struct ClockSnapshot {
    pub time_text: String,
    pub date_text: String,
    pub hour_angle: f32,
    pub minute_angle: f32,
    pub second_angle: f32,
}

impl ClockMode {
    pub fn snapshot(&self) -> ClockSnapshot {
        let now = Local::now();
        let second = now.second() as f32 + now.nanosecond() as f32 / 1_000_000_000.0;
        let minute = now.minute() as f32 + second / 60.0;
        let hour = (now.hour() % 12) as f32 + minute / 60.0;
        ClockSnapshot {
            time_text: now.format("%H:%M:%S").to_string(),
            date_text: now.format("%a, %b %d").to_string(),
            hour_angle: hour / 12.0,
            minute_angle: minute / 60.0,
            second_angle: second / 60.0,
        }
    }
}
