use super::*;
use player_core::appearance::{Design, Scheme, ink, parse_color};
use std::rc::Rc;
impl AppController {
    pub(super) fn library_request(&mut self, request: library_service::Request) {
        if let Some(service) = &self.library_service
            && let Err(e) = service.commands.try_send(request)
        {
            match e {
                std::sync::mpsc::TrySendError::Full(request) => {
                    if matches!(request, library_service::Request::Scan(..)) {
                        self.library_pending = Some(request);
                    } else {
                        self.status = "音乐库服务忙，请稍后重试".into();
                    }
                }
                std::sync::mpsc::TrySendError::Disconnected(_) => {
                    self.status = "音乐库服务已停止".into()
                }
            }
        }
    }
    pub(super) fn scan_library(&mut self) {
        self.library_scan += 1;
        self.library_busy = true;
        if let Some(s) = &self.library_service {
            s.generation
                .store(self.library_scan, std::sync::atomic::Ordering::Relaxed);
        }
        self.library_request(library_service::Request::Scan(
            self.library_scan,
            self.settings.library.clone(),
        ));
    }
    pub(super) fn poll_library(&mut self) {
        if let Some(request) = self.library_pending.take() {
            self.library_request(request);
        }
        let replies: Vec<_> = self
            .library_service
            .as_ref()
            .map(|s| s.replies.try_iter().collect())
            .unwrap_or_default();
        for reply in replies {
            match reply {
                library_service::Reply::Paths(folder, paths) => {
                    self.import_library_paths(folder, paths);
                }
                library_service::Reply::Indexed(id, config, songs, message) => {
                    if config == self.settings.library
                        && (id == self.library_scan || (id == 0 && self.library_songs.is_empty()))
                    {
                        if let Some(t) = &mut self.thumbnails {
                            t.invalidate();
                        }
                        self.library_songs = songs;
                        for song in &mut self.library_songs {
                            if let Some(metadata) = self.settings.online.downloads.get(&song.path) {
                                song.title = metadata.title.clone();
                                song.artist = metadata.artist.clone();
                                song.album = metadata.album.clone();
                            }
                        }
                        if id == self.library_scan {
                            self.library_busy = false;
                        }
                        self.library_message = message;
                        self.refresh_library();
                    }
                }
                library_service::Reply::Error(e) => {
                    self.library_message = e;
                }
            }
        }
    }
    pub(super) fn import_library_paths(&mut self, folder: bool, paths: Vec<PathBuf>) {
        let mut config = self.settings.library.clone();
        for path in paths {
            if folder {
                if !config.roots.contains(&path) {
                    config.roots.push(path);
                }
            } else {
                config.excluded.retain(|p| p != &path);
                if !config.files.contains(&path) {
                    config.files.push(path);
                }
            }
        }
        if let Err(e) = config.validate() {
            self.status = e;
            return;
        }
        self.settings.library = config;
        self.settings_dirty = Some(Instant::now());
        self.refresh_library();
        self.scan_library();
    }
    pub(super) fn refresh_library(&mut self) {
        let query = self.search.trim().to_lowercase();
        let folder = self
            .library_folder
            .checked_sub(1)
            .and_then(|i| self.settings.library.roots.get(i as usize));
        let loose = self.library_folder == self.settings.library.roots.len() as i32 + 1;
        let loose_files: std::collections::BTreeSet<_> =
            self.settings.library.files.iter().collect();
        let excluded: std::collections::BTreeSet<_> =
            self.settings.library.excluded.iter().collect();
        self.library_rows = Rc::new(
            self.library_songs
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    (loose_files.contains(&s.path)
                        || self
                            .settings
                            .library
                            .roots
                            .iter()
                            .any(|p| s.path.starts_with(p)))
                        && !excluded.contains(&s.path)
                        && (!loose || loose_files.contains(&s.path))
                        && folder.is_none_or(|p| s.path.starts_with(p))
                        && (query.is_empty()
                            || format!("{} {} {} {}", s.title, s.artist, s.album, s.path.display())
                                .to_lowercase()
                                .contains(&query))
                })
                .map(|(i, s)| MediaPreview {
                    id: i as i32,
                    title: s.title.clone(),
                    artist: s.artist.clone(),
                    collection: s.album.clone(),
                    duration: if s.seconds > 0 {
                        format_time(Some(Duration::from_secs(s.seconds)))
                    } else {
                        "".into()
                    },
                    cover: -1,
                    artwork: self
                        .thumbnails
                        .as_ref()
                        .map(|t| t.image(&s.path))
                        .unwrap_or_default(),
                })
                .collect(),
        );
        self.library_revision += 1;
    }
    pub(super) fn library_control(&mut self, action: &str, value: &str) -> bool {
        let index = value.parse::<usize>().ok();
        match action {
            "library-folder-add" => self.library_request(library_service::Request::Folder),
            "library-files-add" => self.library_request(library_service::Request::Files),
            "library-rescan" => self.scan_library(),
            "library-cancel" => {
                self.library_scan += 1;
                if let Some(s) = &self.library_service {
                    s.generation
                        .store(self.library_scan, std::sync::atomic::Ordering::Relaxed);
                }
                self.library_pending = None;
                self.library_busy = false;
                self.library_message = "扫描已取消；已完成的索引保留".into();
            }
            "library-folder" => {
                self.library_folder = value.parse().unwrap_or(0);
                self.refresh_library();
            }
            "library-folder-remove" => {
                if self.library_folder > 0
                    && self.library_folder as usize <= self.settings.library.roots.len()
                {
                    self.settings
                        .library
                        .roots
                        .remove(self.library_folder as usize - 1);
                    self.library_folder = 0;
                    self.settings_dirty = Some(Instant::now());
                    self.refresh_library();
                    self.scan_library();
                }
            }
            "library-song-remove" => {
                if let Some(song) = index.and_then(|i| self.library_songs.get(i)).cloned() {
                    self.settings.library.files.retain(|p| p != &song.path);
                    if !self.settings.library.excluded.contains(&song.path) {
                        if self.settings.library.excluded.len() >= 10000 {
                            self.status = "单曲排除已达到 10000 首，请按目录管理".into();
                            return true;
                        }
                        self.settings.library.excluded.push(song.path.clone());
                    }
                    self.settings_dirty = Some(Instant::now());
                    self.library_songs.retain(|s| s.path != song.path);
                    self.refresh_library();
                    self.scan_library();
                }
            }
            "library-play" => {
                if let Some(row) = self
                    .library_rows
                    .iter()
                    .position(|r| Some(r.id as usize) == index)
                {
                    self.queue = self
                        .library_rows
                        .iter()
                        .filter_map(|r| self.library_songs.get(r.id as usize))
                        .map(|s| MediaSource::Local(s.path.clone()))
                        .collect();
                    self.queue_version += 1;
                    self.load_index(row as i32);
                }
            }
            "appearance-design" => {
                self.settings.appearance.design = if value == "0" {
                    Design::Simple
                } else {
                    Design::Fashion
                }
            }
            "appearance-scheme" => {
                self.settings.appearance.scheme = match value {
                    "1" => Scheme::Light,
                    "2" => Scheme::Dark,
                    _ => Scheme::System,
                }
            }
            "appearance-accent-source" => self.settings.appearance.system_accent = value == "0",
            "appearance-accent" => {
                if parse_color(value).is_none() {
                    self.status = "请输入 #RRGGBB 主题色，再点击应用".into();
                    return true;
                }
                self.settings.appearance.accent = value.into();
                self.settings.appearance.system_accent = false;
            }
            "appearance-motion" => self.settings.appearance.reduce_motion = value == "yes",
            _ => return false,
        }
        if action.starts_with("appearance-") {
            self.settings_revision += 1;
            self.settings_dirty = Some(Instant::now());
        }
        true
    }
    pub(super) fn library_view(&self) -> player_ui::view_model::LibraryViewModel {
        let mut folders = vec!["全部音乐".into()];
        folders.extend(
            self.settings
                .library
                .roots
                .iter()
                .map(|p| p.display().to_string()),
        );
        folders.push("单独添加的歌曲".into());
        let current = self.queue.get(self.selected_id as usize);
        let selected = current
            .and_then(|src| {
                if let MediaSource::Local(p) = src {
                    self.library_songs.iter().position(|s| &s.path == p)
                } else {
                    None
                }
            })
            .map(|i| i as i32)
            .unwrap_or(-1);
        player_ui::view_model::LibraryViewModel {
            rows: self.library_rows.clone(),
            revision: self.library_revision,
            folders,
            folder: self.library_folder,
            busy: self.library_busy,
            message: self.library_message.clone(),
            selected,
        }
    }
    pub(super) fn appearance_view(&self) -> player_ui::view_model::AppearanceViewModel {
        let a = &self.settings.appearance;
        let os = self
            .appearance_observer
            .as_ref()
            .map(|o| o.snapshot())
            .unwrap_or_default();
        let color = if a.system_accent {
            os.accent.unwrap_or(0x6875e8)
        } else {
            parse_color(&a.accent).unwrap_or(0x6875e8)
        };
        player_ui::view_model::AppearanceViewModel {
            design: if a.design == Design::Simple { 0 } else { 1 },
            scheme: match a.scheme {
                Scheme::System => 0,
                Scheme::Light => 1,
                Scheme::Dark => 2,
            },
            dark: match a.scheme {
                Scheme::System => os.dark.unwrap_or(true),
                Scheme::Light => false,
                Scheme::Dark => true,
            },
            accent: color,
            ink: ink(color),
            accent_text: player_core::appearance::text_accent(
                color,
                match a.scheme {
                    Scheme::System => os.dark.unwrap_or(true),
                    Scheme::Light => false,
                    Scheme::Dark => true,
                },
            ),
            system_accent: a.system_accent,
            custom_accent: a.accent.clone(),
            reduce_motion: a.reduce_motion,
            os_reduce_motion: os.reduce_motion,
            system_available: os.dark.is_some(),
            library_background_style: a.library_background.style as i32,
            library_background_opacity: a.library_background.opacity as i32,
            library_background_blur: a.library_background.blur as i32,
            library_background_name: a
                .library_background
                .file
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            lyrics_background_style: a.lyrics_background.style as i32,
            lyrics_background_opacity: a.lyrics_background.opacity as i32,
            lyrics_background_blur: a.lyrics_background.blur as i32,
            lyrics_background_name: a
                .lyrics_background
                .file
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            custom_library_background: self
                .backgrounds
                .as_ref()
                .map(|b| b.image(0))
                .unwrap_or_default(),
            custom_lyrics_background: self
                .backgrounds
                .as_ref()
                .map(|b| b.image(1))
                .unwrap_or_default(),
            background_image_revision: self.backgrounds.as_ref().map(|b| b.revision()).unwrap_or(0),
        }
    }
}
