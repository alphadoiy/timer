use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};

use anyhow::Context;
use crossterm::{
    cursor::Hide,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{DefaultTerminal, Terminal, widgets::Clear};
use sysinfo::System;

use crate::{
    animation::Animator,
    cli::Cli,
    modes::{clock::ClockMode, pomodoro::PomodoroState},
    music::{MusicCommand, MusicConfig, MusicEngine, library, queue::TrackQueue},
    render::DashboardView,
    theme::Theme,
    weather_live::configure_location,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeKind {
    Clock,
    Pomodoro,
    Music,
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
    music: MusicEngine,
    music_full_visualizer: bool,
    music_queue_overlay: bool,
    animator: Animator,
    theme: Theme,
    dark_bg: bool,
    system: System,
    system_stats: SystemStats,
    last_system_refresh: Instant,
    should_quit: bool,
}

impl App {
    pub fn new(initial_mode: ModeKind, now: Instant, cli: &Cli) -> Self {
        let mut system = System::new_all();
        system.refresh_cpu_usage();
        system.refresh_memory();

        let music_cfg = MusicConfig::load().merge_cli(&cli.music_options());
        let inputs = library::parse_inputs(cli.music_inputs());
        let tracks = library::build_tracks(&inputs);
        let mut queue = TrackQueue::new(music_cfg.shuffle, music_cfg.repeat_mode);
        queue.load(tracks);

        let mut music = MusicEngine::new(queue, music_cfg.volume);
        if music_cfg.auto_play {
            music.dispatch(MusicCommand::Play);
        }

        Self {
            mode: initial_mode,
            clock: ClockMode,
            pomodoro: PomodoroState::new(now),
            music,
            music_full_visualizer: false,
            music_queue_overlay: false,
            animator: Animator::new(),
            theme: Theme::default(),
            dark_bg: !cli.light_bg(),
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
        if target != ModeKind::Music {
            self.music_full_visualizer = false;
            self.music_queue_overlay = false;
        }
        self.animator.set_animation(from, target, now);
    }

    fn update(&mut self, now: Instant) {
        self.animator.tick(now);
        let completed = self.pomodoro.update(now);
        if completed {
            self.animator.celebrate(now);
        }
        self.music.update();
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
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
                return;
            }
            KeyCode::Tab => {
                self.switch_mode(next_mode(self.mode), now);
                return;
            }
            _ => {}
        }

        match self.mode {
            ModeKind::Clock => match key.code {
                KeyCode::Right => self.switch_mode(next_mode(self.mode), now),
                KeyCode::Left => self.switch_mode(prev_mode(self.mode), now),
                _ => {}
            },
            ModeKind::Pomodoro => match key.code {
                KeyCode::Right => self.switch_mode(next_mode(self.mode), now),
                KeyCode::Left => self.switch_mode(prev_mode(self.mode), now),
                KeyCode::Char(' ') => self.pomodoro.toggle(now),
                KeyCode::Char('r') => self.pomodoro.reset(now),
                KeyCode::Char('b') => self.dark_bg = !self.dark_bg,
                _ => {}
            },
            ModeKind::Music => self.handle_music_key(key.code, key.modifiers),
        }
    }

    fn handle_music_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match code {
            KeyCode::Char(' ') => self.music.dispatch(MusicCommand::Toggle),
            KeyCode::Char('n') => self.music.dispatch(MusicCommand::Next),
            KeyCode::Char('p') => self.music.dispatch(MusicCommand::Prev),
            KeyCode::Char('s') => self.music.dispatch(MusicCommand::ToggleShuffle),
            KeyCode::Char('m') => self.music.toggle_mute(),
            KeyCode::Char('v') => self.music.cycle_visualizer_mode(),
            KeyCode::Char('V') => self.music_full_visualizer = !self.music_full_visualizer,
            KeyCode::Char('Q') => self.music_queue_overlay = !self.music_queue_overlay,
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let volume = self.music.snapshot().volume.saturating_add(5).min(100);
                self.music.dispatch(MusicCommand::SetVolume(volume));
            }
            KeyCode::Char('-') => {
                let volume = self.music.snapshot().volume.saturating_sub(5);
                self.music.dispatch(MusicCommand::SetVolume(volume));
            }
            KeyCode::Right => {
                let step = if modifiers.contains(KeyModifiers::SHIFT) {
                    30
                } else {
                    5
                };
                self.music.dispatch(MusicCommand::Seek(step));
            }
            KeyCode::Left => {
                let step = if modifiers.contains(KeyModifiers::SHIFT) {
                    -30
                } else {
                    -5
                };
                self.music.dispatch(MusicCommand::Seek(step));
            }
            KeyCode::Up => self.music.move_selection(-1),
            KeyCode::Down => self.music.move_selection(1),
            KeyCode::Enter => {
                let idx = self.music.selected_index();
                self.music.select_and_play(idx);
            }
            KeyCode::Char('r') => self
                .music
                .dispatch(MusicCommand::SetRepeat(next_repeat_mode(
                    self.music.snapshot().repeat_mode,
                ))),
            _ => {}
        }
    }

    fn draw(&self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let clock_snapshot = self.clock.snapshot();
        let pomodoro_snapshot = self.pomodoro.snapshot();
        let music_snapshot = self.music.snapshot();
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
                    music: &music_snapshot,
                    music_full_visualizer: self.music_full_visualizer,
                    music_queue_overlay: self.music_queue_overlay,
                    pose,
                    theme: self.theme,
                    dark_bg: self.dark_bg,
                    system: self.system_stats,
                },
                size,
            );
        })?;
        Ok(())
    }

    fn shutdown(&mut self) {
        self.music.shutdown();
    }
}

pub fn run(cli: Cli) -> anyhow::Result<()> {
    configure_location(cli.weather_coords(), cli.auto_location);
    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, cli.initial_mode(), &cli);
    restore_terminal(terminal)?;
    result
}

fn run_app(
    terminal: &mut DefaultTerminal,
    initial_mode: ModeKind,
    cli: &Cli,
) -> anyhow::Result<()> {
    let mut app = App::new(initial_mode, Instant::now(), cli);
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
            while event::poll(Duration::from_millis(0)).context("failed to poll terminal events")? {
                let event = event::read().context("failed to read terminal event")?;
                app.handle_event(event, Instant::now());
            }
        }
    }

    app.shutdown();
    Ok(())
}

fn next_mode(mode: ModeKind) -> ModeKind {
    match mode {
        ModeKind::Clock => ModeKind::Pomodoro,
        ModeKind::Pomodoro => ModeKind::Music,
        ModeKind::Music => ModeKind::Clock,
    }
}

fn prev_mode(mode: ModeKind) -> ModeKind {
    match mode {
        ModeKind::Clock => ModeKind::Music,
        ModeKind::Pomodoro => ModeKind::Clock,
        ModeKind::Music => ModeKind::Pomodoro,
    }
}

fn next_repeat_mode(mode: crate::music::RepeatMode) -> crate::music::RepeatMode {
    match mode {
        crate::music::RepeatMode::Off => crate::music::RepeatMode::All,
        crate::music::RepeatMode::All => crate::music::RepeatMode::One,
        crate::music::RepeatMode::One => crate::music::RepeatMode::Off,
    }
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
