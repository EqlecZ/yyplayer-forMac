//! UI- and platform-independent playback contracts. No playback implementation here.

pub mod engine;
pub mod state;
pub mod types;

pub use engine::{EngineCapabilities, EngineError, PlaybackEngine};
pub use state::{PlaybackPhase, PlaybackSnapshot};
pub use types::{AudioMode, AudioRequest, MediaSource, PlaybackCommand, QueueItem, QueueItemId};
pub mod audio;
pub mod lyrics;
pub mod settings;
pub mod shortcuts;

pub mod appearance;
pub mod library;

pub mod online;
pub mod path_serde;
pub mod typography;
