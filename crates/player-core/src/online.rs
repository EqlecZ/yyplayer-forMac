//! Local preferences only. API credentials and temporary URLs never enter settings.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DownloadMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct OnlineSettings {
    #[serde(with = "crate::path_serde::map")]
    pub downloads: BTreeMap<PathBuf, DownloadMetadata>,
    pub music_enabled: bool,
    pub lyrics_enabled: bool,
    pub keep_history: bool,
    pub history: Vec<String>,
    #[serde(with = "crate::path_serde")]
    pub api_directory: PathBuf,
    #[serde(with = "crate::path_serde")]
    pub download_directory: PathBuf,
    #[serde(with = "crate::path_serde")]
    pub lyric_directory: PathBuf,
    #[serde(with = "crate::path_serde")]
    pub artwork_directory: PathBuf,
    #[serde(with = "crate::path_serde")]
    pub stream_directory: PathBuf,
}
impl Default for OnlineSettings {
    fn default() -> Self {
        Self {
            downloads: BTreeMap::new(),
            music_enabled: false,
            lyrics_enabled: false,
            keep_history: true,
            history: vec![],
            api_directory: PathBuf::new(),
            download_directory: PathBuf::new(),
            lyric_directory: PathBuf::new(),
            artwork_directory: PathBuf::new(),
            stream_directory: PathBuf::new(),
        }
    }
}
impl OnlineSettings {
    pub fn record(&mut self, query: &str) {
        let query = query.trim();
        if !self.keep_history || query.is_empty() || query.chars().count() > 200 {
            return;
        }
        self.history
            .retain(|q| q.to_lowercase() != query.to_lowercase());
        self.history.insert(0, query.into());
        self.history.truncate(20);
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.downloads.len() > 256
            || self.downloads.values().any(|m| {
                [m.title.as_str(), m.artist.as_str(), m.album.as_str()]
                    .iter()
                    .any(|s| s.chars().count() > 512)
            })
        {
            return Err("下载关联元数据超出范围".into());
        }
        if self.history.len() > 20
            || self
                .history
                .iter()
                .any(|q| q.trim().is_empty() || q.chars().count() > 200)
        {
            return Err("搜索历史超出范围".into());
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_submission_and_preferences() {
        let mut p = OnlineSettings::default();
        for i in 0..25 {
            p.record(&format!("歌曲{i}"));
        }
        p.record(" Song ");
        p.record("song");
        assert_eq!(p.history.len(), 20);
        assert_eq!(p.history[0], "song");
        assert!(!p.history.iter().any(|q| q == "Song"));
        p.keep_history = false;
        p.record("new");
        assert_eq!(p.history[0], "song");
        let old: crate::settings::Settings = serde_json::from_str("{\"version\":1}").unwrap();
        assert!(!old.online.music_enabled && !old.online.lyrics_enabled);
        p.validate().unwrap();
    }
}
