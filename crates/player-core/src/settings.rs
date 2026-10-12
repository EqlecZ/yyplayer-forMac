use crate::shortcuts::ShortcutSettings;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum DecodeMode {
    #[default]
    Auto,
    Software,
    HardwarePreferred,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DecodeOptions {
    pub mode: DecodeMode,
    pub threads: u8,
    pub deinterlace: bool,
    pub deband: bool,
}
impl DecodeOptions {
    pub fn hwdec(&self) -> &'static str {
        match self.mode {
            DecodeMode::Auto => "auto-safe",
            DecodeMode::Software => "no",
            DecodeMode::HardwarePreferred => "auto-copy",
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.threads > 32 {
            return Err("解码线程数应为 0–32（0 为自动）".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub online: crate::online::OnlineSettings,
    pub version: u32,
    pub global: DecodeOptions,
    #[serde(with = "crate::path_serde::map")]
    pub folders: BTreeMap<PathBuf, DecodeOptions>,
    #[serde(with = "crate::path_serde::map")]
    pub files: BTreeMap<PathBuf, DecodeOptions>,
    pub shortcuts: ShortcutSettings,
    pub volume: f32,
    pub device: String,
    #[serde(with = "crate::path_serde::vec")]
    pub recent: Vec<PathBuf>,
    pub audio: crate::audio::AudioSettings,
    pub appearance: crate::appearance::Appearance,
    pub library: crate::library::LibrarySettings,
    pub video_library: crate::library::LibrarySettings,
    pub fonts: crate::typography::Fonts,
    pub lyrics: crate::lyrics::LyricStyle,
    pub library_columns: crate::typography::LibraryColumns,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            online: Default::default(),
            version: 1,
            global: DecodeOptions::default(),
            folders: BTreeMap::new(),
            files: BTreeMap::new(),
            shortcuts: ShortcutSettings::default(),
            volume: 70.0,
            device: "auto".into(),
            recent: Vec::new(),
            audio: Default::default(),
            appearance: Default::default(),
            library: Default::default(),
            video_library: Default::default(),
            fonts: Default::default(),
            lyrics: Default::default(),
            library_columns: Default::default(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        self.online.validate()?;
        if self.version != 1 {
            return Err(format!("不支持设置版本 {}", self.version));
        }
        if !self.volume.is_finite() || !(0.0..=100.0).contains(&self.volume) {
            return Err("音量应为 0–100".into());
        }
        self.global.validate()?;
        self.appearance.validate()?;
        self.library.validate()?;
        self.video_library.validate()?;
        self.fonts.validate()?;
        self.lyrics.validate()?;
        self.library_columns.validate()?;
        for options in self.folders.values().chain(self.files.values()) {
            options.validate()?;
        }
        self.shortcuts
            .validate()
            .and_then(|_| self.audio.validate())
    }
    /// Paths must be canonicalized once at the platform boundary; Path::starts_with
    /// compares components, so /movies cannot accidentally match /movies-other.
    pub fn resolve(&self, file: &Path) -> (DecodeOptions, String) {
        if let Some(options) = self.files.get(file) {
            return (options.clone(), "单文件".into());
        }
        if let Some((folder, options)) = self
            .folders
            .iter()
            .filter(|(folder, _)| file.starts_with(folder))
            .max_by_key(|(folder, _)| folder.components().count())
        {
            return (options.clone(), format!("文件夹：{}", folder.display()));
        }
        (self.global.clone(), "全局".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_settings_gain_an_independent_empty_video_library() {
        let mut settings: Settings =
            serde_json::from_str(r#"{"version":1,"library":{"roots":["music"]}}"#).unwrap();
        assert!(settings.video_library.roots.is_empty());
        settings.video_library.roots.push("movies".into());
        let loaded: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(
            loaded.library.roots,
            vec![std::path::PathBuf::from("music")]
        );
        assert_eq!(
            loaded.video_library.roots,
            vec![std::path::PathBuf::from("movies")]
        );
        loaded.validate().unwrap();
    }
    #[test]
    fn nearest_folder_then_file_and_removal() {
        let mut settings = Settings::default();
        let file = PathBuf::from("library/films/a/movie.mkv");
        settings.folders.insert(
            "library/films".into(),
            DecodeOptions {
                mode: DecodeMode::Software,
                ..Default::default()
            },
        );
        settings.folders.insert(
            "library/films/a".into(),
            DecodeOptions {
                mode: DecodeMode::HardwarePreferred,
                ..Default::default()
            },
        );
        assert_eq!(
            settings.resolve(&file).0.mode,
            DecodeMode::HardwarePreferred
        );
        settings
            .files
            .insert(file.clone(), DecodeOptions::default());
        assert_eq!(settings.resolve(&file).0.mode, DecodeMode::Auto);
        settings.files.remove(&file);
        assert_eq!(
            settings.resolve(&file).0.mode,
            DecodeMode::HardwarePreferred
        );
        assert_eq!(
            settings
                .resolve(Path::new("library/films-other/movie.mkv"))
                .0
                .mode,
            DecodeMode::Auto
        );
    }
}
