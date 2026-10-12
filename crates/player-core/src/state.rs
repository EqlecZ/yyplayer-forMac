use std::time::Duration;

use crate::{MediaSource, QueueItemId};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaybackPhase {
    #[default]
    Idle,
    Loading,
    Playing,
    Paused,
    Ended,
    Error,
}

/// Confirmed engine state, kept separate from the UI's demonstration data.
#[derive(Clone, Debug, Default)]
pub struct PlaybackSnapshot {
    pub revision: u64,
    pub generation: u64,
    pub phase: PlaybackPhase,
    pub source: Option<MediaSource>,
    pub current_item: Option<QueueItemId>,
    pub position: Option<Duration>,
    pub duration: Option<Duration>,
    pub audio_output: Option<String>,
    pub exclusive_confirmed: Option<bool>,
    pub ready: bool,
    pub subtitle_font: String,
    pub subtitle_font_overrides: String,
    pub title: String,
    pub video: bool,
    /// Engine observed an active VO, used to retire the presenter after it closes.
    pub video_output_enabled: bool,
    pub speed: f64,
    pub volume: f32,
    pub muted: bool,
    pub devices: Vec<AudioDevice>,
    pub tracks: Vec<MediaTrack>,
    pub chapters: Vec<Chapter>,
    pub info: String,
    pub hwdec: String,
    pub error: String,
    pub runtime: String,
    /// API submitted PCM format; does not prove the physical DAC format.
    pub audio_info: String,
    pub audio_status: String,
    pub eq_filter: String,
    pub audio_log: Vec<String>,
    pub audio_fallback: bool,
    /// Output policy prevents loading or resuming; independent of media identity.
    pub audio_blocked: bool,
    pub audio_source_rate: Option<u64>,
    pub audio_output_rate: Option<u64>,
    /// Actual demuxer / codec and packet bitrate, never a service's quality label.
    pub media_format: String,
    pub audio_codec: String,
    pub audio_backend: String,
    pub audio_bitrate: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Debug, Default)]
pub struct MediaTrack {
    pub id: i64,
    pub kind: String,
    pub label: String,
    pub selected: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Chapter {
    pub title: String,
    pub seconds: f64,
}
