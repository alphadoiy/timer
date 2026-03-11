use rand::RngExt;

use super::{RepeatMode, TrackMeta};

#[derive(Debug, Clone)]
pub struct TrackQueue {
    tracks: Vec<TrackMeta>,
    current_index: Option<usize>,
    shuffle: bool,
    repeat_mode: RepeatMode,
}

impl TrackQueue {
    pub fn new(shuffle: bool, repeat_mode: RepeatMode) -> Self {
        Self {
            tracks: Vec::new(),
            current_index: None,
            shuffle,
            repeat_mode,
        }
    }

    pub fn load(&mut self, tracks: Vec<TrackMeta>) {
        self.tracks = tracks;
        self.current_index = if self.tracks.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Append tracks to the end of the queue without resetting the current
    /// playback position.
    pub fn append(&mut self, tracks: Vec<TrackMeta>) {
        if self.tracks.is_empty() {
            self.load(tracks);
        } else {
            self.tracks.extend(tracks);
        }
    }

    pub fn tracks(&self) -> &[TrackMeta] {
        &self.tracks
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    pub fn current(&self) -> Option<&TrackMeta> {
        self.current_index.and_then(|idx| self.tracks.get(idx))
    }

    pub fn select(&mut self, index: usize) -> Option<&TrackMeta> {
        if index >= self.tracks.len() {
            return None;
        }
        self.current_index = Some(index);
        self.current()
    }

    pub fn next(&mut self, manual: bool) -> Option<&TrackMeta> {
        let next = self.next_index(manual)?;
        self.current_index = Some(next);
        self.current()
    }

    pub fn prev(&mut self) -> Option<&TrackMeta> {
        let prev = self.prev_index()?;
        self.current_index = Some(prev);
        self.current()
    }

    pub fn toggle_shuffle(&mut self) -> bool {
        self.shuffle = !self.shuffle;
        self.shuffle
    }

    pub fn set_repeat_mode(&mut self, repeat_mode: RepeatMode) {
        self.repeat_mode = repeat_mode;
    }

    pub fn repeat_mode(&self) -> RepeatMode {
        self.repeat_mode
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    pub fn set_current_duration(&mut self, duration: std::time::Duration) {
        if let Some(idx) = self.current_index
            && let Some(track) = self.tracks.get_mut(idx)
        {
            track.duration = Some(duration);
        }
    }

    fn next_index(&self, manual: bool) -> Option<usize> {
        if self.tracks.is_empty() {
            return None;
        }
        let current = self.current_index.unwrap_or(0);

        if self.repeat_mode == RepeatMode::One && !manual {
            return Some(current);
        }

        if self.shuffle && self.tracks.len() > 1 {
            let mut rng = rand::rng();
            let mut candidate = current;
            while candidate == current {
                candidate = rng.random_range(0..self.tracks.len());
            }
            return Some(candidate);
        }

        let at_end = current + 1 >= self.tracks.len();
        if at_end {
            return match self.repeat_mode {
                RepeatMode::All => Some(0),
                RepeatMode::Off | RepeatMode::One => None,
            };
        }

        Some(current + 1)
    }

    fn prev_index(&self) -> Option<usize> {
        if self.tracks.is_empty() {
            return None;
        }

        let current = self.current_index.unwrap_or(0);
        if self.shuffle && self.tracks.len() > 1 {
            let mut rng = rand::rng();
            let mut candidate = current;
            while candidate == current {
                candidate = rng.random_range(0..self.tracks.len());
            }
            return Some(candidate);
        }

        if current == 0 {
            return match self.repeat_mode {
                RepeatMode::All => Some(self.tracks.len().saturating_sub(1)),
                RepeatMode::Off | RepeatMode::One => Some(0),
            };
        }

        Some(current - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn track(id: u64) -> TrackMeta {
        TrackMeta {
            id,
            title: format!("track-{id}"),
            artist: "Unknown".to_string(),
            duration: Some(Duration::from_secs(60)),
            provider: super::super::ProviderKind::Local,
            path_or_url: format!("/tmp/{id}.mp3"),
        }
    }

    #[test]
    fn repeat_all_wraps_to_start() {
        let mut queue = TrackQueue::new(false, RepeatMode::All);
        queue.load(vec![track(1), track(2)]);
        queue.select(1);
        let next = queue.next(false).expect("expected wrapped track");
        assert_eq!(next.id, 1);
    }

    #[test]
    fn repeat_one_keeps_same_track_on_auto_advance() {
        let mut queue = TrackQueue::new(false, RepeatMode::One);
        queue.load(vec![track(1), track(2)]);
        queue.select(1);
        let next = queue.next(false).expect("expected same track");
        assert_eq!(next.id, 2);
    }

    #[test]
    fn shuffle_picks_another_track_when_possible() {
        let mut queue = TrackQueue::new(true, RepeatMode::Off);
        queue.load(vec![track(1), track(2), track(3)]);
        queue.select(0);
        let next = queue.next(true).expect("expected track");
        assert_ne!(next.id, 1);
    }

    #[test]
    fn append_extends_without_resetting_index() {
        let mut queue = TrackQueue::new(false, RepeatMode::Off);
        queue.load(vec![track(1), track(2)]);
        queue.select(1);
        queue.append(vec![track(3), track(4)]);
        assert_eq!(queue.current_index(), Some(1));
        assert_eq!(queue.tracks().len(), 4);
    }

    #[test]
    fn append_to_empty_sets_index() {
        let mut queue = TrackQueue::new(false, RepeatMode::Off);
        queue.append(vec![track(1)]);
        assert_eq!(queue.current_index(), Some(0));
    }
}
