use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};

use anyhow::Context;
use crossterm::{
    cursor::Hide,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{DefaultTerminal, Terminal, widgets::Clear};
use sysinfo::System;

use crate::{
    animation::Animator,
    cli::Cli,
    modes::{clock::ClockMode, pomodoro::PomodoroState},
    render::DashboardView,
    theme::Theme,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeKind {
    Clock,
    Pomodoro,
}

pub trait Mode {
    fn update(&mut self, _now: Instant) {}
}

impl Mode for ClockMode {}

impl Mode for PomodoroState {
    fn update(&mut self, now: Instant) {
        PomodoroState::update(self, now);
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub memory_used_mib: u64,
    pub memory_total_mib: u64,
}

pub struct App {
    mode: ModeKind,
    clock: ClockMode,
    pomodoro: PomodoroState,
    animator: Animator,
    theme: Theme,
    system: System,
    system_stats: SystemStats,
    last_system_refresh: Instant,
    should_quit: bool,
}

impl App {
    pub fn new(initial_mode: ModeKind, now: Instant) -> Self {
        let mut system = System::new_all();
        system.refresh_cpu_usage();
        system.refresh_memory();
        Self {
            mode: initial_mode,
            clock: ClockMode,
            pomodoro: PomodoroState::new(now),
            animator: Animator::new(),
            theme: Theme::default(),
            system_stats: collect_system_stats(&system),
            system,
            last_system_refresh: now,
            should_quit: false,
        }
    }

    fn switch_mode(&mut self, target: ModeKind, now: Instant) {
        if self.mode == target {
            return;
        }
        let from = self.mode;
        self.mode = target;
        self.animator.set_animation(from, target, now);
    }

    fn update(&mut self, now: Instant) {
        self.animator.tick(now);
        let completed = self.pomodoro.update(now);
        if completed {
            self.animator.celebrate(now);
        }
        if now.duration_since(self.last_system_refresh) >= Duration::from_secs(1) {
            self.system.refresh_cpu_usage();
            self.system.refresh_memory();
            self.system_stats = collect_system_stats(&self.system);
            self.last_system_refresh = now;
        }
    }

    fn handle_event(&mut self, event: Event, now: Instant) {
        let Event::Key(key) = event else {
            return;
        };
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Tab | KeyCode::Right => self.switch_mode(next_mode(self.mode), now),
            KeyCode::Left => self.switch_mode(prev_mode(self.mode), now),
            KeyCode::Char(' ') => self.pomodoro.toggle(now),
            KeyCode::Char('r') => self.pomodoro.reset(now),
            _ => {}
        }
    }

    fn draw(&self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let clock_snapshot = self.clock.snapshot();
        let pomodoro_snapshot = self.pomodoro.snapshot();
        let pose = self
            .animator
            .current_pose(self.mode, pomodoro_snapshot, Instant::now());

        terminal.draw(|frame| {
            let size = frame.area();
            frame.render_widget(Clear, size);
            frame.render_widget(
                DashboardView {
                    mode: self.mode,
                    clock: &clock_snapshot,
                    pomodoro: pomodoro_snapshot,
                    pose,
                    theme: self.theme,
                    system: self.system_stats,
                },
                size,
            );
        })?;
        Ok(())
    }
}

pub fn run(cli: Cli) -> anyhow::Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, cli.initial_mode());
    restore_terminal(terminal)?;
    result
}

fn run_app(terminal: &mut DefaultTerminal, initial_mode: ModeKind) -> anyhow::Result<()> {
    let mut app = App::new(initial_mode, Instant::now());
    let frame_budget = Duration::from_millis(16);
    let mut next_frame_at = Instant::now();

    while !app.should_quit {
        let now = Instant::now();
        app.update(now);
        app.draw(terminal)?;

        next_frame_at += frame_budget;
        let wait = next_frame_at.saturating_duration_since(Instant::now());

        if event::poll(wait).context("failed to poll terminal events")? {
            let event = event::read().context("failed to read terminal event")?;
            app.handle_event(event, Instant::now());
            while event::poll(Duration::from_millis(0))
                .context("failed to poll terminal events")?
            {
                let event = event::read().context("failed to read terminal event")?;
                app.handle_event(event, Instant::now());
            }
        }
    }

    Ok(())
}

fn next_mode(mode: ModeKind) -> ModeKind {
    match mode {
        ModeKind::Clock => ModeKind::Pomodoro,
        ModeKind::Pomodoro => ModeKind::Clock,
    }
}

fn prev_mode(mode: ModeKind) -> ModeKind {
    next_mode(mode)
}

fn setup_terminal() -> anyhow::Result<DefaultTerminal> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)
        .context("failed to initialize terminal screen state")?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))
        .context("failed to create terminal")?;
    terminal.hide_cursor().context("failed to hide cursor")?;
    Ok(terminal)
}

fn restore_terminal(
    mut terminal: Terminal<ratatui::backend::CrosstermBackend<Stdout>>,
) -> anyhow::Result<()> {
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("failed to leave alternate screen")?;
    terminal.show_cursor().context("failed to show cursor")?;
    Ok(())
}

fn collect_system_stats(system: &System) -> SystemStats {
    let used_mib = system.used_memory() / (1024 * 1024);
    let total_mib = system.total_memory() / (1024 * 1024);
    SystemStats {
        cpu_usage: system.global_cpu_usage(),
        memory_used_mib: used_mib,
        memory_total_mib: total_mib,
    }
}
