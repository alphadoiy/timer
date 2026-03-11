use std::time::Duration;

use super::{MusicSnapshot, RepeatMode};

pub fn mode_label(snapshot: &MusicSnapshot) -> &'static str {
    snapshot.state.label()
}

pub fn repeat_label(mode: RepeatMode) -> &'static str {
    match mode {
        RepeatMode::Off => "OFF",
        RepeatMode::All => "ALL",
        RepeatMode::One => "ONE",
    }
}

pub fn duration_text(position: Duration, total: Option<Duration>) -> String {
    format!(
        "{} / {}",
        fmt(position),
        total.map_or_else(|| "LIVE".to_string(), fmt)
    )
}

pub fn queue_lines(snapshot: &MusicSnapshot, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    if snapshot.queue.is_empty() {
        out.push("(empty queue)".to_string());
        return out;
    }

    let start = snapshot.selected_index.saturating_sub(max / 2);
    let end = (start + max).min(snapshot.queue.len());

    for (idx, track) in snapshot.queue[start..end].iter().enumerate() {
        let absolute_idx = start + idx;
        let pointer = if Some(absolute_idx) == snapshot.current_index {
            '▶'
        } else {
            ' '
        };
        let selector = if absolute_idx == snapshot.selected_index {
            '>'
        } else {
            ' '
        };
        let icon = track.provider.icon();
        out.push(format!("{selector}{pointer} {icon} {}", track.title));
    }

    out
}

fn fmt(duration: Duration) -> String {
    let secs = duration.as_secs();
    format!("{:02}:{:02}", secs / 60, secs % 60)
}
