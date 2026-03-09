use super::*;

impl DashboardView<'_> {
    pub(super) fn render_music_body(&self, area: Rect, buf: &mut Buffer) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(6),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Min(6),
                Constraint::Length(2),
            ])
            .split(area);

        self.render_cliamp_title(rows[0], buf);
        self.render_cliamp_track(rows[1], buf);
        self.render_cliamp_time_status(rows[2], buf);
        self.render_cliamp_visualizer(rows[3], buf);
        self.render_cliamp_seek(rows[4], buf);
        self.render_cliamp_controls(rows[5], buf);
        self.render_cliamp_playlist_header(rows[6], buf);
        self.render_cliamp_playlist(rows[7], buf);
        self.render_cliamp_help(rows[8], buf);
    }

    fn render_cliamp_title(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(Line::from(vec![
            Span::styled(
                "C L I A M P",
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  (timer edition)", Style::default().fg(self.theme.subtext)),
        ]))
        .alignment(Alignment::Left)
        .render(area, buf);
    }

    fn render_cliamp_track(&self, area: Rect, buf: &mut Buffer) {
        let track = self
            .music
            .current_index
            .and_then(|idx| self.music.queue.get(idx))
            .map(|t| t.title.as_str())
            .unwrap_or("No track loaded");
        let artist = self
            .music
            .current_index
            .and_then(|idx| self.music.queue.get(idx))
            .map(|t| t.artist.as_str())
            .unwrap_or("Unknown");

        let line1 = Line::from(vec![
            Span::styled("♫ ", Style::default().fg(Color::LightYellow)),
            Span::styled(track, Style::default().fg(Color::LightYellow)),
        ]);
        let line2 = Line::from(Span::styled(
            format!("  {artist}"),
            Style::default().fg(self.theme.subtext),
        ));
        Paragraph::new(vec![line1, line2]).render(area, buf);
    }

    fn render_cliamp_time_status(&self, area: Rect, buf: &mut Buffer) {
        let time = music_ui::duration_text(self.music.position, self.music.duration);
        let status = match self.music.state {
            crate::music::PlaybackState::Playing => "▶ Playing",
            crate::music::PlaybackState::Paused => "⏸ Paused",
            crate::music::PlaybackState::Buffering => "◌ Buffering",
            crate::music::PlaybackState::Ended => "■ Ended",
            crate::music::PlaybackState::Error(_) => "⚠ Error",
            crate::music::PlaybackState::Idle | crate::music::PlaybackState::Stopped => "■ Stopped",
        };
        let mut text = time.clone();
        let space = area
            .width
            .saturating_sub((time.len() + status.len()) as u16) as usize;
        text.push_str(&" ".repeat(space.max(1)));
        text.push_str(status);
        Paragraph::new(Line::from(Span::styled(
            text,
            Style::default().fg(Color::White),
        )))
        .render(area, buf);
    }

    fn render_cliamp_visualizer(&self, area: Rect, buf: &mut Buffer) {
        let h = area.height as usize;
        let w = area.width as usize;
        if h == 0 || w == 0 {
            return;
        }
        let bands = self.music.spectrum_bands;
        let styled = match self.music.visualizer_mode {
            crate::music::VisualizerMode::Bricks => {
                styled_from_plain(render_bricks(bands, w, h), h)
            }
            crate::music::VisualizerMode::Columns => {
                styled_from_plain(render_columns(bands, w, h), h)
            }
            crate::music::VisualizerMode::Wave => {
                styled_from_plain(render_braille_wave(&self.music.wave_samples, w, h), h)
            }
            crate::music::VisualizerMode::Scatter => {
                render_braille_scatter_styled(bands, w, h, self.music.visualizer_frame)
            }
            crate::music::VisualizerMode::Flame => styled_from_plain(
                render_braille_flame(bands, w, h, self.music.visualizer_frame),
                h,
            ),
            crate::music::VisualizerMode::Matrix => {
                render_matrix_styled(bands, w, h, self.music.visualizer_frame)
            }
            crate::music::VisualizerMode::Binary => {
                render_binary_styled(bands, w, h, self.music.visualizer_frame)
            }
        };

        for (idx, line) in styled.iter().enumerate() {
            Paragraph::new(styled_line_to_spans(line)).render(
                Rect {
                    x: area.x,
                    y: area.y + idx as u16,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
        }
    }

    fn render_cliamp_seek(&self, area: Rect, buf: &mut Buffer) {
        let width = area.width as usize;
        if width == 0 {
            return;
        }

        let progress = self
            .music
            .duration
            .filter(|d| !d.is_zero())
            .map(|total| (self.music.position.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0));
        let line = build_visual_progress_line(
            width,
            progress,
            self.music.position.as_secs_f32(),
        );
        Paragraph::new(line).render(area, buf);
    }

    fn render_cliamp_controls(&self, area: Rect, buf: &mut Buffer) {
        let left = format!(
            "EQ [{}]  SHUF:{}  REP:{}",
            self.music.visualizer_mode.label(),
            if self.music.shuffle { "ON" } else { "OFF" },
            music_ui::repeat_label(self.music.repeat_mode)
        );
        let right = format!(
            "VOL {:>3}% {}",
            self.music.volume,
            if self.music.muted { "[M]" } else { "" }
        );
        let gap = area
            .width
            .saturating_sub((left.len() + right.len()) as u16)
            .max(1) as usize;
        Paragraph::new(Line::from(vec![
            Span::styled(left, Style::default().fg(Color::LightCyan)),
            Span::raw(" ".repeat(gap)),
            Span::styled(right, Style::default().fg(Color::Green)),
        ]))
        .render(area, buf);
    }

    fn render_cliamp_playlist_header(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(Line::from(vec![
            Span::styled(
                "PLAYLIST",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {} tracks", self.music.queue.len()),
                Style::default().fg(self.theme.subtext),
            ),
        ]))
        .render(area, buf);
    }

    fn render_cliamp_playlist(&self, area: Rect, buf: &mut Buffer) {
        let lines = music_ui::queue_lines(self.music, area.height as usize);
        let content = lines
            .into_iter()
            .map(|line| {
                let style = if line.contains('▶') {
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD)
                } else if line.starts_with('>') {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(line, style))
            })
            .collect::<Vec<_>>();
        Paragraph::new(content).render(area, buf);
    }

    fn render_cliamp_help(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(Line::from(vec![
            Span::styled("[Space] ", Style::default().fg(self.theme.subtext)),
            Span::styled("⏯ ", Style::default().fg(Color::White)),
            Span::styled("[n/p] ", Style::default().fg(self.theme.subtext)),
            Span::styled("track ", Style::default().fg(Color::White)),
            Span::styled("[v] ", Style::default().fg(self.theme.subtext)),
            Span::styled("vis ", Style::default().fg(Color::White)),
            Span::styled("[V] ", Style::default().fg(self.theme.subtext)),
            Span::styled("full ", Style::default().fg(Color::White)),
            Span::styled("[Q] ", Style::default().fg(self.theme.subtext)),
            Span::styled("queue ", Style::default().fg(Color::White)),
            Span::styled("[m] ", Style::default().fg(self.theme.subtext)),
            Span::styled("mute ", Style::default().fg(Color::White)),
            Span::styled("[q] quit", Style::default().fg(self.theme.subtext)),
        ]))
        .render(area, buf);
    }

    pub(super) fn render_music_full_visualizer(&self, area: Rect, buf: &mut Buffer) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .split(area);

        let now_playing = self
            .music
            .current_index
            .and_then(|idx| self.music.queue.get(idx))
            .map(|t| t.title.clone())
            .unwrap_or_else(|| "No track loaded".to_string());

        Paragraph::new(vec![
            Line::from(Span::styled(
                now_playing,
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                music_ui::duration_text(self.music.position, self.music.duration),
                Style::default().fg(Color::White),
            )),
        ])
        .alignment(Alignment::Center)
        .render(rows[0], buf);

        self.render_cliamp_seek(rows[1], buf);
        self.render_cliamp_visualizer(rows[2], buf);
        Paragraph::new(Line::from(Span::styled(
            "[V] exit fullscreen  [v] switch mode  [Space] play/pause",
            Style::default().fg(self.theme.subtext),
        )))
        .alignment(Alignment::Center)
        .render(rows[3], buf);
    }

    pub(super) fn render_music_queue_overlay(&self, area: Rect, buf: &mut Buffer) {
        let overlay = centered_rect(area, 72, 68);
        Block::default()
            .title(Line::from(Span::styled(
                " Queue Manager ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.accent_soft))
            .style(Style::default().bg(self.theme.shadow))
            .render(overlay, buf);
        let inner = Rect {
            x: overlay.x + 1,
            y: overlay.y + 1,
            width: overlay.width.saturating_sub(2),
            height: overlay.height.saturating_sub(2),
        };
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(inner);
        let lines = music_ui::queue_lines(self.music, rows[0].height as usize);
        let content = lines
            .into_iter()
            .map(|line| {
                let style = if line.contains('▶') {
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD)
                } else if line.starts_with('>') {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(line, style))
            })
            .collect::<Vec<_>>();
        Paragraph::new(content).render(rows[0], buf);
        Paragraph::new(Line::from(Span::styled(
            "[Q] close  [↑/↓] select  [Enter] play",
            Style::default().fg(self.theme.subtext),
        )))
        .alignment(Alignment::Center)
        .render(rows[1], buf);
    }

    pub(super) fn render_music_visual_panel(&self, area: Rect, buf: &mut Buffer) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(5),
                Constraint::Length(2),
            ])
            .split(area);
        let now_playing = self
            .music
            .current_index
            .and_then(|idx| self.music.queue.get(idx))
            .map(|t| t.title.clone())
            .unwrap_or_else(|| "No track loaded".to_string());
        Paragraph::new(vec![
            Line::from(Span::styled(
                "Now Playing",
                Style::default().fg(self.theme.subtext),
            )),
            Line::from(Span::styled(
                now_playing,
                Style::default()
                    .fg(self.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                music_ui::duration_text(self.music.position, self.music.duration),
                Style::default().fg(self.theme.accent),
            )),
        ])
        .block(box_block("Current", self.theme))
        .alignment(Alignment::Center)
        .render(rows[0], buf);

        let queue_lines =
            music_ui::queue_lines(self.music, rows[1].height.saturating_sub(2) as usize);
        let queue_text = queue_lines
            .into_iter()
            .map(|line| Line::from(Span::styled(line, Style::default().fg(self.theme.text))))
            .collect::<Vec<_>>();
        Paragraph::new(queue_text)
            .block(box_block("Queue", self.theme))
            .render(rows[1], buf);

        Paragraph::new(Line::from(Span::styled(
            "Space toggle | n/p next/prev | s shuffle | r repeat | m mute",
            Style::default().fg(self.theme.subtext),
        )))
        .alignment(Alignment::Center)
        .render(rows[2], buf);
    }

    pub(super) fn render_music_info_panel(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(Line::from(Span::styled(
                " Music Readout ",
                Style::default()
                    .fg(self.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            )))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.outline));
        let inner = block.inner(area);
        block.render(area, buf);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Min(3),
            ])
            .split(inner);

        Paragraph::new(vec![
            Line::from(Span::styled(
                music_ui::mode_label(self.music),
                Style::default().fg(self.theme.highlight),
            )),
            Line::from(Span::styled(
                format!("{} tracks", self.music.queue.len()),
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("Playback", self.theme))
        .alignment(Alignment::Center)
        .render(rows[0], buf);

        Paragraph::new(vec![
            Line::from(Span::styled(
                format!("Volume {}%", self.music.volume),
                Style::default().fg(self.theme.accent),
            )),
            Line::from(Span::styled(
                if self.music.muted {
                    "Muted"
                } else {
                    "Output On"
                },
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("Audio", self.theme))
        .alignment(Alignment::Center)
        .render(rows[1], buf);

        Paragraph::new(vec![
            Line::from(Span::styled(
                format!("Repeat {}", music_ui::repeat_label(self.music.repeat_mode)),
                Style::default().fg(self.theme.highlight),
            )),
            Line::from(Span::styled(
                format!("Shuffle {}", if self.music.shuffle { "On" } else { "Off" }),
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("Queue", self.theme))
        .alignment(Alignment::Center)
        .render(rows[2], buf);

        Paragraph::new(vec![
            Line::from(Span::styled(
                music_ui::duration_text(self.music.position, self.music.duration),
                Style::default().fg(self.theme.text),
            )),
            Line::from(Span::styled(
                "Left/Right seek",
                Style::default().fg(self.theme.subtext),
            )),
        ])
        .block(box_block("Timeline", self.theme))
        .alignment(Alignment::Center)
        .render(rows[3], buf);

        let err = self
            .music
            .last_error
            .clone()
            .unwrap_or_else(|| "No errors".to_string());
        Paragraph::new(Line::from(Span::styled(
            err,
            Style::default().fg(self.theme.subtext),
        )))
        .block(box_block("Health", self.theme))
        .alignment(Alignment::Center)
        .render(rows[4], buf);
    }
}
