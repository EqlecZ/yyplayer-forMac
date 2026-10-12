use super::*;
use player_core::audio::{BandKind, EqBand, EqPreset, OutputMode};
use player_core::{AudioMode, AudioRequest};
use player_ui::view_model::AudioViewModel;

// Source properties come from the bounded tag worker or the Engine snapshot.
// Compressed PCM decode format cannot stand in for a file's source bit depth.
fn technical_info(
    assets: &music_assets::Assets,
    snapshot: &player_core::PlaybackSnapshot,
) -> String {
    let format = if assets.file_type == "MPEG" && !snapshot.audio_codec.is_empty() {
        snapshot.audio_codec.to_uppercase()
    } else if !assets.file_type.is_empty() {
        assets.file_type.clone()
    } else if !snapshot.media_format.is_empty() {
        snapshot.media_format.to_uppercase()
    } else if !snapshot.audio_codec.is_empty() {
        snapshot.audio_codec.to_uppercase()
    } else {
        "格式未知".into()
    };
    let bitrate = assets
        .bitrate_kbps
        .map(|v| format!("{v} kbps"))
        .or_else(|| {
            snapshot
                .audio_bitrate
                .map(|v| format!("≈{} kbps", v.saturating_add(500) / 1000))
        })
        .unwrap_or_else(|| "码率未知".into());
    let rate = assets
        .sample_rate
        .map(u64::from)
        .or(snapshot.audio_source_rate)
        .map(|v| {
            if v % 1000 == 0 {
                format!("{} kHz", v / 1000)
            } else if v % 100 == 0 {
                format!("{:.1} kHz", v as f64 / 1000.0)
            } else {
                format!("{v} Hz")
            }
        })
        .unwrap_or_else(|| "采样率未知".into());
    let depth = assets
        .bit_depth
        .map(|v| format!("{v} bit"))
        .unwrap_or_else(|| "源位深未知".into());
    let status = if snapshot.audio_status.is_empty() {
        "输出状态未知"
    } else {
        &snapshot.audio_status
    };
    let backend = if snapshot.audio_backend.is_empty()
        || status
            .to_ascii_lowercase()
            .contains(&snapshot.audio_backend.to_ascii_lowercase())
    {
        String::new()
    } else {
        format!(" · {}", snapshot.audio_backend.to_uppercase())
    };
    format!("{format} | {bitrate} | {rate} | {depth} | {status}{backend}")
}

impl AppController {
    pub(super) fn audio_path(&self) -> Option<&std::path::Path> {
        match self.queue.get(self.selected_id as usize) {
            Some(MediaSource::Local(p)) => Some(p),
            _ => None,
        }
    }
    pub(super) fn apply_output(&mut self) {
        self.send(PlaybackCommand::ApplyAudioRequest(AudioRequest {
            device_id: self.settings.device.clone(),
            volume_percent: self.settings.volume,
            preserve_rate: self.settings.audio.preserve_rate,
            mode: match self.settings.audio.mode {
                OutputMode::Shared => AudioMode::Shared,
                OutputMode::PreferExclusive => AudioMode::PreferExclusive,
                OutputMode::StrictExclusive => AudioMode::StrictExclusive,
            },
        }));
    }
    pub(super) fn apply_eq(&mut self) {
        let eq = self
            .settings
            .audio
            .resolve(self.audio_path(), &self.settings.device)
            .0;
        self.send(PlaybackCommand::ApplyEq(eq));
    }
    pub(super) fn refresh_eq_draft(&mut self) {
        self.eq_draft = match self.eq_scope {
            0 => self.settings.audio.global_eq.clone(),
            1 => self
                .settings
                .audio
                .device_presets
                .get(&self.settings.device)
                .and_then(|n| self.settings.audio.presets.get(n))
                .cloned()
                .unwrap_or_else(|| self.settings.audio.global_eq.clone()),
            _ => {
                self.settings
                    .audio
                    .resolve(self.audio_path(), &self.settings.device)
                    .0
            }
        };
        self.settings_revision += 1;
    }
    pub(super) fn request_asset(&mut self, request: music_assets::Request) {
        self.cancel_hold();
        if matches!(&request, music_assets::Request::Read(..)) {
            self.pending_asset = None;
        }
        if let Some(service) = &self.assets_service {
            match service.commands.try_send(request) {
                Ok(()) => {}
                Err(std::sync::mpsc::TrySendError::Full(request))
                    if matches!(&request, music_assets::Request::Read(..)) =>
                {
                    self.pending_asset = Some(request)
                }
                Err(error) => self.status = format!("后台文件任务暂未提交：{error}"),
            }
        }
    }
    pub(super) fn read_assets(&mut self, source: &MediaSource) {
        self.asset_revision += 1;
        self.assets = Default::default();
        self.cover = Default::default();
        self.asset_projection_revision += 1;
        self.lyric_rows = Default::default();
        self.plain_lyrics = Default::default();
        if let MediaSource::Local(path) = source {
            let lyrics = self.settings.audio.lyric_files.get(path).cloned();
            self.request_asset(music_assets::Request::Read(
                self.asset_revision,
                path.clone(),
                lyrics,
            ));
        }
        self.refresh_eq_draft();
    }
    pub(super) fn poll_assets(&mut self) {
        if let Some(request) = self.pending_asset.take()
            && let Some(service) = &self.assets_service
        {
            match service.commands.try_send(request) {
                Ok(()) => {}
                Err(std::sync::mpsc::TrySendError::Full(request)) => {
                    self.pending_asset = Some(request)
                }
                Err(error) => self.status = format!("资源读取服务不可用：{error}"),
            }
        }
        let replies: Vec<_> = self
            .assets_service
            .as_ref()
            .map(|s| s.replies.try_iter().collect())
            .unwrap_or_default();
        for reply in replies {
            match reply {
                music_assets::Reply::Assets(id, assets) if id == self.asset_revision => {
                    let mut assets = *assets;
                    if let Some(song) = &self.online.playing {
                        assets.title = song.title.clone();
                        assets.artist = song.artist.clone();
                        assets.album = song.album.clone();
                        if assets.cover.is_none() {
                            assets.cover_color = self.assets.cover_color;
                        }
                        if assets.lyrics.lines.is_empty() && assets.lyrics.plain.is_empty() {
                            assets.lyrics = self.assets.lyrics.clone();
                        }
                    }
                    if let Some(path) = self.audio_path()
                        && let Some(metadata) = self.settings.online.downloads.get(path)
                    {
                        assets.title = metadata.title.clone();
                        assets.artist = metadata.artist.clone();
                        assets.album = metadata.album.clone();
                    }
                    self.assets = assets;
                    if let Some((width, height, bytes)) = self.assets.cover.take() {
                        let buffer =
                            slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                                &bytes, width, height,
                            );
                        self.cover = slint::Image::from_rgba8(buffer);
                    }
                    if !self.assets.warning.is_empty() {
                        self.status = self.assets.warning.clone();
                    }
                    self.lyric_rows = std::rc::Rc::new(
                        self.assets
                            .lyrics
                            .lines
                            .iter()
                            .map(|l| (l.text.clone(), l.milliseconds as i32))
                            .collect(),
                    );
                    self.asset_projection_revision += 1;
                    self.plain_lyrics = std::rc::Rc::new(self.assets.lyrics.plain.clone());
                    // A new projection revision marks completed artwork/lyrics, without changing request identity.
                }
                music_assets::Reply::Lyrics(id, media, path, lyrics)
                    if id == self.asset_revision && self.audio_path() == Some(media.as_path()) =>
                {
                    self.assets.lyrics = lyrics;
                    self.lyric_rows = std::rc::Rc::new(
                        self.assets
                            .lyrics
                            .lines
                            .iter()
                            .map(|l| (l.text.clone(), l.milliseconds as i32))
                            .collect(),
                    );
                    self.asset_projection_revision += 1;
                    self.plain_lyrics = std::rc::Rc::new(self.assets.lyrics.plain.clone());
                    self.settings.audio.lyric_files.insert(media, path);
                    self.save();
                }
                music_assets::Reply::Eq(eq) => {
                    self.eq_draft = eq;
                    self.settings_revision += 1;
                    self.status = "预设已导入草稿；请选择范围，再应用".into();
                }
                music_assets::Reply::Message(message) => self.status = message,
                _ => {}
            }
        }
    }
    pub(super) fn audio_control(&mut self, action: &str, value: &str) -> bool {
        let number = value.parse::<f64>().ok().filter(|v| v.is_finite());
        match action {
            "output-retry" => self.apply_output(),
            "music-toggle" if self.navigation.current() == 4 => {
                self.go_back();
            }
            "music-toggle" | "music-detail" => {
                if !self.engine.snapshot().video {
                    self.visit_page(4);
                }
            }
            "music-browse" => {
                self.visit_page(0);
            }
            "output-mode" => {
                self.settings.audio.mode = match value {
                    "1" => OutputMode::PreferExclusive,
                    "2" => OutputMode::StrictExclusive,
                    _ => OutputMode::Shared,
                };
                self.apply_output();
                self.settings_revision += 1;
                self.save();
            }
            "preserve-rate" => {
                self.settings.audio.preserve_rate = value == "yes";
                self.apply_output();
                self.settings_revision += 1;
                self.save();
            }
            "eq-scope" => {
                self.eq_scope = number.unwrap_or(0.0) as i32;
                self.refresh_eq_draft();
            }
            "eq-enabled" => self.eq_draft.enabled = value == "yes",
            "eq-headroom" => {
                self.eq_draft.auto_headroom = value == "yes";
                self.settings_revision += 1;
            }
            "eq-name" => self.eq_draft.name = value.into(),
            "eq-preamp" => self.eq_draft.preamp = number.unwrap_or(f64::NAN),
            "eq-band" => {
                let parts: Vec<_> = value.split(':').collect();
                if parts.len() == 3
                    && let Ok(index) = parts[0].parse::<usize>()
                    && let Some(band) = self.eq_draft.bands.get_mut(index)
                {
                    let number = parts[2].parse::<f64>().unwrap_or(f64::NAN);
                    match parts[1] {
                        "f" => band.frequency = number,
                        "g" => band.gain = number,
                        "q" => band.q = number,
                        "k" => {
                            band.kind = match parts[2] {
                                "1" => BandKind::LowShelf,
                                "2" => BandKind::HighShelf,
                                _ => BandKind::Peak,
                            }
                        }
                        _ => {}
                    }
                }
            }
            "eq-add" => {
                if self.eq_draft.bands.len() < 32 {
                    self.eq_draft.bands.push(EqBand {
                        frequency: 1000.0,
                        gain: 0.0,
                        q: 1.414,
                        kind: BandKind::Peak,
                    });
                    self.settings_revision += 1;
                }
            }
            "eq-remove" => {
                if let Ok(i) = value.parse::<usize>()
                    && i < self.eq_draft.bands.len()
                {
                    self.eq_draft.bands.remove(i);
                    self.settings_revision += 1;
                }
            }
            "eq-flat" => {
                self.eq_draft = EqPreset::default();
                self.settings_revision += 1;
            }
            "eq-import" => self.request_asset(music_assets::Request::ImportEq),
            "eq-export" => {
                self.request_asset(music_assets::Request::ExportEq(self.eq_draft.clone()));
            }
            "eq-preset" => {
                self.preset_index = number.unwrap_or(-1.0) as i32;
                if let Some(eq) = self
                    .settings
                    .audio
                    .presets
                    .values()
                    .nth(self.preset_index as usize)
                {
                    self.eq_draft = eq.clone();
                    self.settings_revision += 1;
                }
            }
            "eq-store" => {
                if let Err(e) = self.eq_draft.validate() {
                    self.status = e;
                } else if self.settings.audio.presets.len() < 256
                    || self
                        .settings
                        .audio
                        .presets
                        .contains_key(&self.eq_draft.name)
                {
                    self.settings
                        .audio
                        .presets
                        .insert(self.eq_draft.name.clone(), self.eq_draft.clone());
                    self.settings_revision += 1;
                    self.save();
                } else {
                    self.status = "最多保存 256 个 EQ 预设".into();
                }
            }
            "eq-delete" => {
                if let Some(name) = self
                    .settings
                    .audio
                    .presets
                    .keys()
                    .nth(self.preset_index as usize)
                    .cloned()
                {
                    self.settings.audio.presets.remove(&name);
                    self.settings
                        .audio
                        .device_presets
                        .retain(|_, preset| preset != &name);
                    self.preset_index = -1;
                    self.refresh_eq_draft();
                    self.apply_eq();
                    self.save();
                } else {
                    self.status = "请先选择一个命名预设".into();
                }
            }
            "eq-apply" => {
                if let Err(e) = self.eq_draft.validate() {
                    self.status = e;
                } else {
                    let previous = self.settings.audio.clone();
                    match self.eq_scope {
                        0 => self.settings.audio.global_eq = self.eq_draft.clone(),
                        1 => {
                            if self.settings.device == "auto" {
                                self.status =
                                    "设备 EQ 请先选择具体输出设备，避免系统默认设备变化后绑定错误"
                                        .into();
                                return true;
                            }
                            self.settings
                                .audio
                                .presets
                                .insert(self.eq_draft.name.clone(), self.eq_draft.clone());
                            self.settings
                                .audio
                                .device_presets
                                .insert(self.settings.device.clone(), self.eq_draft.name.clone());
                        }
                        2 => {
                            if let Some(path) = self.audio_path().map(std::path::Path::to_path_buf)
                            {
                                self.settings
                                    .audio
                                    .file_eq
                                    .insert(path, self.eq_draft.clone());
                            } else {
                                self.status = "单文件 EQ 需要本地媒体".into();
                                return true;
                            }
                        }
                        _ => return true,
                    }
                    if let Err(error) = self.settings.audio.validate() {
                        self.settings.audio = previous;
                        self.status = error;
                        return true;
                    }
                    self.apply_eq();
                    self.settings_revision += 1;
                    self.save();
                }
            }
            "eq-clear" => {
                match self.eq_scope {
                    0 => self.settings.audio.global_eq = Default::default(),
                    1 => {
                        self.settings
                            .audio
                            .device_presets
                            .remove(&self.settings.device);
                    }
                    2 => {
                        if let Some(path) = self.audio_path().map(std::path::Path::to_path_buf) {
                            self.settings.audio.file_eq.remove(&path);
                        }
                    }
                    _ => {}
                }
                self.refresh_eq_draft();
                self.apply_eq();
                self.save();
            }
            "lyrics-import" => {
                if let Some(path) = self.audio_path().map(std::path::Path::to_path_buf) {
                    self.request_asset(music_assets::Request::Lyrics(self.asset_revision, path));
                }
            }
            "lyrics-offset" => {
                if let Some(ms) = number
                    && (-600000.0..=600000.0).contains(&ms)
                {
                    self.settings.audio.lyric_offset_ms = ms as i64;
                    self.settings_revision += 1;
                    self.settings_dirty = Some(Instant::now());
                }
            }
            "lyrics-seek" => {
                if let Some(ms) = number {
                    self.send(PlaybackCommand::Seek(Duration::from_millis(
                        (ms as i64 - self.settings.audio.lyric_offset_ms).max(0) as u64,
                    )));
                }
            }
            _ => return false,
        }
        true
    }
    pub(super) fn audio_view(&self) -> AudioViewModel {
        let position = self
            .engine
            .snapshot()
            .position
            .map_or(0, |p| p.as_millis() as i64);
        let (_, origin) = self
            .settings
            .audio
            .resolve(self.audio_path(), &self.settings.device);
        AudioViewModel {
            preview: self.preview,
            cover: self.cover.clone(),
            cover_color: self.assets.cover_color,
            lyric_style: self.settings.lyrics.clone(),
            lyric_style_revision: self.settings_revision,
            asset_revision: self.asset_projection_revision,
            lyrics: self.lyric_rows.clone(),
            plain_lyrics: self.plain_lyrics.clone(),
            active_lyric: self
                .assets
                .lyrics
                .active(position, self.settings.audio.lyric_offset_ms)
                .map_or(-1, |i| i as i32),
            album: self.assets.album.clone(),
            status: self.engine.snapshot().audio_status.clone(),
            technical_info: technical_info(&self.assets, self.engine.snapshot()),
            origin,
            mode: match self.settings.audio.mode {
                OutputMode::Shared => 0,
                OutputMode::PreferExclusive => 1,
                OutputMode::StrictExclusive => 2,
            },
            preserve_rate: self.settings.audio.preserve_rate,
            offset_ms: self.settings.audio.lyric_offset_ms as i32,
            eq_scope: self.eq_scope,
            eq_name: self.eq_draft.name.clone(),
            eq_enabled: self.eq_draft.enabled,
            preamp: self.eq_draft.preamp.to_string(),
            auto_headroom: self.eq_draft.auto_headroom,
            headroom: format!(
                "自动余量：实际前置增益 {:.1} dB",
                self.eq_draft.applied_preamp()
            ),
            bands: self
                .eq_draft
                .bands
                .iter()
                .map(|b| {
                    (
                        b.frequency.to_string(),
                        b.gain as f32,
                        b.q.to_string(),
                        match b.kind {
                            BandKind::Peak => 0,
                            BandKind::LowShelf => 1,
                            BandKind::HighShelf => 2,
                        },
                    )
                })
                .collect(),
            presets: self.settings.audio.presets.keys().cloned().collect(),
            preset_index: self.preset_index,
            binding: self
                .settings
                .audio
                .device_presets
                .get(&self.settings.device)
                .cloned()
                .unwrap_or("未绑定，使用全局".into()),
        }
    }
}

#[cfg(test)]
mod technical_tests {
    use super::*;

    #[test]
    fn source_depth_and_actual_output_are_not_guessed() {
        let mut snapshot = player_core::PlaybackSnapshot {
            audio_codec: "mp3".into(),
            audio_bitrate: Some(128_000),
            audio_source_rate: Some(44_100),
            audio_status: "WASAPI 共享输出".into(),
            ..Default::default()
        };
        let assets = music_assets::Assets::default();
        assert_eq!(
            technical_info(&assets, &snapshot),
            "MP3 | ≈128 kbps | 44.1 kHz | 源位深未知 | WASAPI 共享输出"
        );
        snapshot.audio_status = "WASAPI 独占已确认".into();
        let assets = music_assets::Assets {
            file_type: "FLAC".into(),
            bitrate_kbps: Some(912),
            sample_rate: Some(96_000),
            bit_depth: Some(24),
            ..Default::default()
        };
        assert_eq!(
            technical_info(&assets, &snapshot),
            "FLAC | 912 kbps | 96 kHz | 24 bit | WASAPI 独占已确认"
        );
        assert_eq!(
            technical_info(
                &music_assets::Assets::default(),
                &player_core::PlaybackSnapshot::default()
            ),
            "格式未知 | 码率未知 | 采样率未知 | 源位深未知 | 输出状态未知"
        );
    }
}
