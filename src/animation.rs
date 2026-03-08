use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::app::ModeKind;
use crate::modes::pomodoro::{PhaseKind, PomodoroSnapshot};

#[derive(Debug, Clone, Copy)]
pub struct SpritePose {
    pub dial_offset_x: i16,
    pub dial_offset_y: i16,
    pub radius_scale: f32,
    pub tilt: f32,
    pub second_sweep: f32,
    pub minute_sweep: f32,
    pub hour_sweep: f32,
    pub ring_pulse: f32,
    pub pulse: f32,
    pub progress: f32,
    pub transition_mix: f32,
}

impl Default for SpritePose {
    fn default() -> Self {
        Self {
            dial_offset_x: 0,
            dial_offset_y: 0,
            radius_scale: 1.0,
            tilt: 0.0,
            second_sweep: 0.0,
            minute_sweep: 0.0,
            hour_sweep: 0.0,
            ring_pulse: 0.0,
            pulse: 0.0,
            progress: 1.0,
            transition_mix: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Transition {
    pub from: ModeKind,
    pub to: ModeKind,
    pub started_at: Instant,
    pub duration: Duration,
}

#[derive(Debug, Clone, Copy)]
pub struct Celebration {
    pub started_at: Instant,
    pub duration: Duration,
}

#[derive(Debug)]
pub struct Animator {
    transition: Option<Transition>,
    celebration: Option<Celebration>,
}

impl Default for Animator {
    fn default() -> Self {
        Self::new()
    }
}

impl Animator {
    pub fn new() -> Self {
        Self {
            transition: None,
            celebration: None,
        }
    }

    pub fn set_animation(&mut self, from: ModeKind, to: ModeKind, now: Instant) {
        self.transition = Some(Transition {
            from,
            to,
            started_at: now,
            duration: Duration::from_millis(520),
        });
    }

    pub fn celebrate(&mut self, now: Instant) {
        self.celebration = Some(Celebration {
            started_at: now,
            duration: Duration::from_millis(900),
        });
    }

    pub fn tick(&mut self, now: Instant) {
        if let Some(transition) = self.transition
            && now.duration_since(transition.started_at) >= transition.duration
        {
            self.transition = None;
        }
        if let Some(celebration) = self.celebration
            && now.duration_since(celebration.started_at) >= celebration.duration
        {
            self.celebration = None;
        }
    }

    pub fn current_pose(
        &self,
        mode: ModeKind,
        pomodoro: PomodoroSnapshot,
        now: Instant,
    ) -> SpritePose {
        let mut pose = match mode {
            ModeKind::Clock => self.clock_pose(now),
            ModeKind::Pomodoro => self.pomodoro_pose(pomodoro, now),
        };

        if let Some(transition) = self.transition {
            let t = normalized_progress(now, transition.started_at, transition.duration);
            pose = self.apply_transition(pose, transition, t);
        }

        if let Some(celebration) = self.celebration {
            let t = normalized_progress(now, celebration.started_at, celebration.duration);
            pose = self.apply_celebration(pose, t);
        }

        pose.progress = pomodoro.progress;
        pose
    }

    pub fn is_transitioning(&self) -> bool {
        self.transition.is_some()
    }

    fn clock_pose(&self, now: Instant) -> SpritePose {
        let _t = seconds_f32(now);
        SpritePose {
            dial_offset_x: 0,
            dial_offset_y: 0,
            radius_scale: 1.0,
            tilt: 0.0,
            // Clock mode uses a true linear sweep; the live snapshot already includes sub-second precision.
            second_sweep: 0.0,
            minute_sweep: 0.0,
            hour_sweep: 0.0,
            ring_pulse: 0.0,
            pulse: 0.0,
            progress: 1.0,
            transition_mix: 0.0,
        }
    }

    fn pomodoro_pose(&self, pomodoro: PomodoroSnapshot, now: Instant) -> SpritePose {
        let t = seconds_f32(now);
        let urgency = 1.0 - pomodoro.progress;
        let phase_weight = match pomodoro.phase {
            PhaseKind::Work => 1.0,
            PhaseKind::ShortBreak => 0.45,
            PhaseKind::LongBreak => 0.25,
        };
        let pulse = if pomodoro.running || pomodoro.completed {
            let whole = pomodoro.remaining.as_secs_f32().max(0.0);
            0.14 + (1.0 - (whole.fract() - 0.5).abs() * 2.0) * (0.18 + urgency * 0.24)
        } else {
            0.03
        };

        SpritePose {
            dial_offset_x: 0,
            dial_offset_y: -(pulse * 1.6).round() as i16,
            radius_scale: 1.0 + pulse * 0.025,
            tilt: wave(t * (0.4 + urgency * 0.7)) * 0.025 * phase_weight,
            second_sweep: urgency * 0.05,
            minute_sweep: urgency * 0.025,
            hour_sweep: urgency * 0.015,
            ring_pulse: pulse,
            pulse,
            progress: pomodoro.progress,
            transition_mix: 0.0,
        }
    }

    fn apply_transition(
        &self,
        mut pose: SpritePose,
        _transition: Transition,
        t: f32,
    ) -> SpritePose {
        pose.radius_scale *= 0.94 + ease_out_back(t) * 0.06;
        pose.tilt += wave(t * std::f32::consts::TAU * 1.5) * 0.12;
        pose.pulse += 0.15 * (1.0 - (t - 0.5).abs() * 2.0).max(0.0);
        pose.transition_mix = t;
        pose
    }

    fn apply_celebration(&self, mut pose: SpritePose, t: f32) -> SpritePose {
        let hop = if t < 0.5 {
            ease_out_back(t / 0.5)
        } else {
            1.0 - ease_out_bounce((t - 0.5) / 0.5)
        };
        pose.dial_offset_y -= (hop * 2.0).round() as i16;
        pose.ring_pulse += 0.35;
        pose.tilt += wave(t * std::f32::consts::TAU * 4.0) * 0.05;
        pose.pulse += 0.2;
        pose
    }
}

fn normalized_progress(now: Instant, started_at: Instant, duration: Duration) -> f32 {
    (now.duration_since(started_at).as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

fn seconds_f32(now: Instant) -> f32 {
    let _ = now;
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f32()
}

fn wave(v: f32) -> f32 {
    v.sin()
}

fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

fn ease_out_bounce(t: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_completes_after_duration() {
        let now = Instant::now();
        let mut animator = Animator::new();
        animator.set_animation(ModeKind::Clock, ModeKind::Pomodoro, now);
        animator.tick(now + Duration::from_millis(600));
        assert!(!animator.is_transitioning());
    }
}
