#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RainIntensity {
    Drizzle,
    Light,
    Heavy,
    Storm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnowIntensity {
    Light,
    Medium,
    Heavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FogIntensity {
    Light,
    Medium,
    Heavy,
}
