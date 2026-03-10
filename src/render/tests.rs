use super::*;

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;

    use super::*;
    use crate::{
        app::ModeKind,
        modes::clock::ClockSnapshot,
        music::{MusicSnapshot, PlaybackState, VisualizerMode},
        theme::Theme,
    };

    #[test]
    fn dashboard_renders_key_text() {
        let area = Rect::new(0, 0, 120, 36);
        let mut buffer = Buffer::empty(area);
        let view = DashboardView {
            mode: ModeKind::Pomodoro,
            clock: &ClockSnapshot {
                time_text: "12:34:56".into(),
                date_text: "Sun, Mar 08".into(),
                hour_angle: 0.0,
                minute_angle: 0.5,
                second_angle: 0.75,
            },
            pomodoro: PomodoroSnapshot {
                phase: crate::modes::pomodoro::PhaseKind::Work,
                remaining: std::time::Duration::from_secs(12 * 60),
                progress: 0.5,
                running: true,
                completed: false,
                cycle: 2,
            },
            music: &MusicSnapshot {
                state: PlaybackState::Idle,
                ..MusicSnapshot::default()
            },
            music_full_visualizer: false,
            music_queue_overlay: false,
            pose: SpritePose::default(),
            theme: Theme::default(),
            dark_bg: true,
            system: SystemStats {
                cpu_usage: 13.5,
                memory_used_mib: 1024,
                memory_total_mib: 8192,
            },
        };
        view.render(area, &mut buffer);
        let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("Braille Dial"));
        assert!(text.contains("12:34:56"));
        assert!(text.contains("CPU"));
    }

    #[test]
    fn truncate_handles_multibyte_chars() {
        let mut s = "ｱｲｳｴｵ".to_string();
        truncate_to_char_count(&mut s, 3);
        assert_eq!(s, "ｱｲｳ");
    }

    #[test]
    fn music_visualizer_modes_render_without_panic() {
        let area = Rect::new(0, 0, 120, 36);
        for mode in [
            VisualizerMode::Bricks,
            VisualizerMode::Columns,
            VisualizerMode::Wave,
            VisualizerMode::Scatter,
            VisualizerMode::Flame,
            VisualizerMode::Matrix,
            VisualizerMode::Binary,
        ] {
            let mut buffer = Buffer::empty(area);
            let snapshot = MusicSnapshot {
                state: PlaybackState::Playing,
                visualizer_mode: mode,
                spectrum_bands: [0.4; crate::music::NUM_BANDS],
                wave_samples: (0..2048)
                    .map(|i| ((i as f32) * 0.01).sin())
                    .collect::<Vec<_>>(),
                ..MusicSnapshot::default()
            };
            let view = DashboardView {
                mode: ModeKind::Music,
                clock: &ClockSnapshot {
                    time_text: "12:34:56".into(),
                    date_text: "Sun, Mar 08".into(),
                    hour_angle: 0.0,
                    minute_angle: 0.5,
                    second_angle: 0.75,
                },
                pomodoro: PomodoroSnapshot {
                    phase: crate::modes::pomodoro::PhaseKind::Work,
                    remaining: std::time::Duration::from_secs(12 * 60),
                    progress: 0.5,
                    running: true,
                    completed: false,
                    cycle: 2,
                },
                music: &snapshot,
                music_full_visualizer: false,
                music_queue_overlay: false,
                pose: SpritePose::default(),
                theme: Theme::default(),
                dark_bg: true,
                system: SystemStats {
                    cpu_usage: 13.5,
                    memory_used_mib: 1024,
                    memory_total_mib: 8192,
                },
            };
            view.render(area, &mut buffer);
        }
    }
}
