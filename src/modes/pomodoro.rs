use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseKind {
    Work,
    ShortBreak,
    LongBreak,
}

impl PhaseKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Work => "Focus Sprint",
            Self::ShortBreak => "Short Break",
            Self::LongBreak => "Long Break",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PomodoroSnapshot {
    pub phase: PhaseKind,
    pub remaining: Duration,
    pub progress: f32,
    pub running: bool,
    pub completed: bool,
    pub cycle: u32,
}

#[derive(Debug, Clone)]
pub struct PomodoroState {
    phase: PhaseKind,
    work_duration: Duration,
    short_break_duration: Duration,
    long_break_duration: Duration,
    remaining: Duration,
    running_since: Option<Instant>,
    last_tick: Instant,
    running: bool,
    completed: bool,
    cycle: u32,
}

impl PomodoroState {
    pub fn new(now: Instant) -> Self {
        let work_duration = Duration::from_secs(25 * 60);
        Self {
            phase: PhaseKind::Work,
            work_duration,
            short_break_duration: Duration::from_secs(5 * 60),
            long_break_duration: Duration::from_secs(15 * 60),
            remaining: work_duration,
            running_since: None,
            last_tick: now,
            running: false,
            completed: false,
            cycle: 1,
        }
    }

    pub fn update(&mut self, now: Instant) -> bool {
        self.last_tick = now;
        if !self.running {
            return false;
        }

        let Some(running_since) = self.running_since else {
            self.running_since = Some(now);
            return false;
        };

        let elapsed = now.saturating_duration_since(running_since);
        let total = self.phase_duration();
        if elapsed >= total {
            self.remaining = Duration::ZERO;
            self.running = false;
            self.running_since = None;
            self.completed = true;
            return true;
        }

        self.remaining = total - elapsed;
        false
    }

    pub fn toggle(&mut self, now: Instant) {
        if self.completed {
            self.advance_phase(now);
            self.start(now);
            return;
        }

        if self.running {
            self.pause(now);
        } else {
            self.start(now);
        }
    }

    pub fn reset(&mut self, now: Instant) {
        self.remaining = self.phase_duration();
        self.running = false;
        self.running_since = None;
        self.completed = false;
        self.last_tick = now;
    }

    pub fn advance_phase(&mut self, now: Instant) {
        self.phase = self.next_phase();
        if self.phase == PhaseKind::Work {
            self.cycle += 1;
        }
        self.remaining = self.phase_duration();
        self.completed = false;
        self.running = false;
        self.running_since = None;
        self.last_tick = now;
    }

    pub fn snapshot(&self) -> PomodoroSnapshot {
        let total = self.phase_duration().as_secs_f32();
        let remaining = self.remaining.as_secs_f32();
        PomodoroSnapshot {
            phase: self.phase,
            remaining: self.remaining,
            progress: if total <= f32::EPSILON {
                0.0
            } else {
                (remaining / total).clamp(0.0, 1.0)
            },
            running: self.running,
            completed: self.completed,
            cycle: self.cycle,
        }
    }

    fn start(&mut self, now: Instant) {
        self.running = true;
        let base = self.phase_duration().saturating_sub(self.remaining);
        self.running_since = Some(now - base);
        self.last_tick = now;
    }

    fn pause(&mut self, now: Instant) {
        self.update(now);
        self.running = false;
        self.running_since = None;
    }

    fn next_phase(&self) -> PhaseKind {
        match self.phase {
            PhaseKind::Work => {
                if self.cycle % 4 == 0 {
                    PhaseKind::LongBreak
                } else {
                    PhaseKind::ShortBreak
                }
            }
            PhaseKind::ShortBreak | PhaseKind::LongBreak => PhaseKind::Work,
        }
    }

    fn phase_duration(&self) -> Duration {
        match self.phase {
            PhaseKind::Work => self.work_duration,
            PhaseKind::ShortBreak => self.short_break_duration,
            PhaseKind::LongBreak => self.long_break_duration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_resume_keeps_remaining_time() {
        let now = Instant::now();
        let mut state = PomodoroState::new(now);
        state.toggle(now);
        state.update(now + Duration::from_secs(10));
        let before_pause = state.snapshot().remaining;
        state.toggle(now + Duration::from_secs(10));
        state.toggle(now + Duration::from_secs(20));
        state.update(now + Duration::from_secs(30));
        assert_eq!(
            state.snapshot().remaining,
            before_pause.saturating_sub(Duration::from_secs(10))
        );
    }

    #[test]
    fn completion_marks_phase_done() {
        let now = Instant::now();
        let mut state = PomodoroState::new(now);
        state.toggle(now);
        let completed = state.update(now + Duration::from_secs(25 * 60));
        assert!(completed);
        let snapshot = state.snapshot();
        assert!(snapshot.completed);
        assert!(!snapshot.running);
        assert_eq!(snapshot.remaining, Duration::ZERO);
    }
}
