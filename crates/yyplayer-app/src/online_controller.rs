use super::{
    AppController,
    online::{
        self, Event, Job,
        repository::{Song, eligible},
    },
};
use player_core::{MediaSource, PlaybackCommand};
use player_ui::view_model::{MediaPreview, OnlineViewModel};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
#[derive(Default)]
pub(super) struct State {
    pub service: Option<online::Service>,
    pub config: Option<online::config::Config>,
    pub provider: usize,
    pub source: usize,
    pub query: String,
    pub submitted: String,
    pub page: usize,
    pub more: bool,
    pub lyric_mode: bool,
    pub songs: Vec<Song>,
    pub result_bytes: usize,
    pub result_limited: bool,
    pub rows: Rc<Vec<MediaPreview>>,
    pub revision: u64,
    pub busy: bool,
    pub busy_lanes: [bool; 4],
    pub message: String,
    pub ids: [u64; 4],
    pub selected: Option<usize>,
    pub options: Vec<(bool, String)>,
    pub download: bool,
    pub download_play: bool,
    pub playing: Option<Song>,
    pub playing_source: Option<MediaSource>,
    pub active_id: u64,
    pub streaming: bool,
    pub proxy: Option<online::proxy::Proxy>,
    pub retired: Vec<online::proxy::Proxy>,
    pub lyric_target: Option<(MediaSource, u64)>,
    pub covers: BTreeMap<String, slint::Image>,
    pub art_pending: BTreeSet<String>,
    pub art_done: BTreeSet<String>,
    pub needs_reload: bool,
}
impl AppController {
    fn online_providers(&self) -> impl Iterator<Item = &online::config::Provider> {
        self.online
            .config
            .iter()
            .flat_map(|c| c.providers.iter())
            .filter(|p| {
                if self.online.lyric_mode {
                    p.lyric_search.is_some() && p.lyric.is_some()
                } else {
                    p.search.is_some() && p.stream.is_some()
                }
            })
    }
    fn online_provider(&self) -> Option<online::config::Provider> {
        self.online_providers().nth(self.online.provider).cloned()
    }
    fn online_source(&self) -> String {
        self.online_provider()
            .and_then(|p| p.sources.keys().nth(self.online.source).cloned())
            .unwrap_or_default()
    }
    fn online_job(&mut self, lane: usize, job: Job, new: bool) -> bool {
        let Some(service) = &self.online.service else {
            return false;
        };
        if new {
            self.online.ids[lane] = service.cancel(lane);
            self.online.busy_lanes[lane] = false;
        }
        let foreground = !matches!(
            &job,
            Job::Artwork(..) | Job::Directory(..) | Job::OpenDirectory(..)
        );
        if foreground {
            self.online.busy_lanes[lane] = true;
        }
        self.online.busy = self.online.busy_lanes.iter().any(|b| *b);
        if let Err(e) = service.submit(lane, self.online.ids[lane], job) {
            self.online.message = e;
            self.online.busy_lanes[lane] = false;
            self.online.busy = self.online.busy_lanes.iter().any(|b| *b);
            return false;
        }
        true
    }
    pub(super) fn online_reload(&mut self) {
        let dirs = online::Directories::from_settings(&self.settings.online);
        self.online.busy = true;
        self.online.needs_reload = false;
        self.online_job(0, Job::Reload(dirs.api), true);
    }
    fn online_search(&mut self, page: usize) {
        let allowed = if self.online.lyric_mode {
            self.settings.online.lyrics_enabled
        } else {
            self.settings.online.music_enabled
        };
        if !allowed {
            self.online.message = "请先在设置中开启对应联网服务".into();
            return;
        }
        let Some(p) = self.online_provider() else {
            self.online.message = "API配置尚未就绪".into();
            return;
        };
        let query = if page == 1 {
            self.online.query.trim().to_owned()
        } else {
            self.online.submitted.clone()
        };
        if query.is_empty() || query.chars().count() > 200 {
            self.online.message = "请输入1–200字的关键词".into();
            return;
        }
        if page == 1 && !self.online.lyric_mode {
            self.settings.online.record(&query);
            self.save_with_notice(false);
        }
        self.online.submitted = query.clone();
        self.online.page = page;
        self.online.busy = true;
        self.online.more = false;
        self.online.selected = None;
        self.online.options.clear();
        if page == 1 {
            self.online.songs.clear();
            self.online.result_bytes = 0;
            self.online.result_limited = false;
            self.online.rows = Default::default();
            self.online.revision += 1;
            self.online.covers.clear();
            self.online.art_done.clear();
            self.online.art_pending.clear();
            if let Some(s) = &self.online.service {
                self.online.ids[2] = s.cancel(2);
            }
        }
        self.online.message = "正在搜索…".into();
        self.online_job(
            0,
            Job::Search(p, self.online_source(), query, page, self.online.lyric_mode),
            true,
        );
    }
    fn online_refresh_rows(&mut self) {
        self.online.rows = Rc::new(
            self.online
                .songs
                .iter()
                .enumerate()
                .map(|(i, s)| MediaPreview {
                    id: i as i32,
                    title: s.title.clone(),
                    artist: s.artist.clone(),
                    collection: s.album.clone(),
                    duration: s
                        .duration_ms
                        .map(|n| format!("{}:{:02}", n / 60000, n / 1000 % 60))
                        .unwrap_or_default(),
                    cover: 0,
                    artwork: self
                        .online
                        .covers
                        .get(&s.key())
                        .cloned()
                        .unwrap_or_default(),
                })
                .collect(),
        );
        self.online.revision += 1;
    }
    fn online_play(&mut self, index: usize, quality: Option<usize>, download: bool) {
        if !self.settings.online.music_enabled {
            self.online.message = "联网音乐已关闭".into();
            return;
        }
        let Some(song) = self.online.songs.get(index).cloned() else {
            return;
        };
        let Some(p) = self
            .online
            .config
            .as_ref()
            .and_then(|c| c.providers.iter().find(|p| p.id == song.provider))
            .cloned()
        else {
            return;
        };
        let quality = quality.or_else(|| p.qualities.iter().position(|q| eligible(&song, q)));
        let Some(quality) = quality else {
            self.online.message = "此歌曲暂无适用音质".into();
            return;
        };
        if !p.qualities.get(quality).is_some_and(|q| eligible(&song, q)) {
            return;
        }
        self.online.download = download;
        self.online.busy = true;
        self.online.message = if download {
            "正在下载…"
        } else {
            "正在获取播放地址…"
        }
        .into();
        self.online.selected = None;
        self.online_job(
            1,
            Job::Play(
                p,
                song,
                quality,
                online::Directories::from_settings(&self.settings.online),
                download,
                self.settings.online.lyrics_enabled,
            ),
            true,
        );
    }
    pub(super) fn online_control(&mut self, action: &str, value: &str) -> bool {
        let index = value.parse::<usize>().ok();
        match action {
            "online-query" => self.online.query = value.chars().take(201).collect(),
            "online-search" => self.online_search(1),
            "online-more"
                if self.navigation.current() == 6
                    && self.online.more
                    && !self.online.busy
                    && self.online.selected.is_none() =>
            {
                self.online_search(self.online.page + 1)
            }
            "online-provider" => {
                self.online.provider = index.unwrap_or(0);
                self.online.source = 0;
                if let Some(service) = &self.online.service {
                    self.online.ids[1] = service.cancel(1);
                    self.online.busy_lanes[1] = false;
                }
                self.online_reset_search();
            }
            "online-source" => {
                self.online.source = index.unwrap_or(0);
                if let Some(service) = &self.online.service {
                    self.online.ids[1] = service.cancel(1);
                    self.online.busy_lanes[1] = false;
                }
                self.online_reset_search();
            }
            "online-reload" => self.online_reload(),
            "online-history" => {
                if let Some(q) = index
                    .and_then(|i| self.settings.online.history.get(i))
                    .cloned()
                {
                    self.online.query = q;
                    self.online_search(1);
                }
            }
            "online-history-delete" => {
                if let Some(i) = index
                    && i < self.settings.online.history.len()
                {
                    self.settings.online.history.remove(i);
                    self.save_with_notice(false);
                }
            }
            "online-history-clear" => {
                self.settings.online.history.clear();
                self.save_with_notice(false);
            }
            "online-history-keep" => {
                self.settings.online.keep_history = value == "yes";
                self.save_with_notice(false);
            }
            "online-music-enabled" => {
                self.settings.online.music_enabled = value == "yes";
                if let Some(s) = &self.online.service {
                    s.set_permissions(
                        self.settings.online.music_enabled,
                        self.settings.online.lyrics_enabled,
                    );
                }
                if value != "yes" {
                    if let Some(s) = &self.online.service {
                        self.online.ids[1] = s.cancel(1);
                        self.online.ids[2] = s.cancel(2);
                        self.online.busy_lanes[1] = false;
                        if !self.online.lyric_mode {
                            self.online.ids[0] = s.cancel(0);
                            self.online.busy_lanes[0] = false;
                        }
                    }
                    self.online.busy = self.online.busy_lanes.iter().any(|b| *b);
                    self.online.art_pending.clear();
                    if self.online.streaming {
                        self.send(PlaybackCommand::Stop);
                        self.online_stop_proxy();
                    }
                }
                self.save();
            }
            "online-lyrics-enabled" => {
                self.settings.online.lyrics_enabled = value == "yes";
                if let Some(s) = &self.online.service {
                    s.set_permissions(
                        self.settings.online.music_enabled,
                        self.settings.online.lyrics_enabled,
                    );
                }
                if value != "yes" {
                    if let Some(s) = &self.online.service {
                        self.online.ids[3] = s.cancel(3);
                    }
                    if self.online.lyric_mode {
                        self.online_reset_search();
                    }
                }
                self.save();
            }
            "online-directory" => {
                if let Some(i) = index {
                    self.online_job(3, Job::Directory(i), true);
                }
            }
            "online-directory-open" => {
                let path = online::Directories::from_settings(&self.settings.online).api;
                self.online_job(3, Job::OpenDirectory(path), true);
            }
            "online-directory-reset" => {
                if let Some(path) = index.and_then(|i| self.online_directory_mut(i)) {
                    *path = Default::default();
                    self.online.needs_reload = true;
                    self.save();
                }
            }
            "online-cancel" => {
                self.online_cancel();
                self.online.message = "操作已取消".into();
                self.online.selected = None;
            }
            "online-play" => {
                if let Some(i) = index {
                    if self.online.lyric_mode {
                        self.online_choose_lyrics(i);
                    } else {
                        self.online_play(i, None, false);
                    }
                }
            }
            "online-menu" => {
                if let Some(i) = index
                    && i < self.online.songs.len()
                {
                    self.online.selected = Some(i);
                    self.online.options.clear();
                    self.online.download = false;
                }
            }
            "online-options-close" => {
                if self.online.selected.is_some() && self.online.busy_lanes[1] {
                    if let Some(s) = &self.online.service {
                        self.online.ids[1] = s.cancel(1);
                    }
                    self.online.busy_lanes[1] = false;
                    self.online.busy = self.online.busy_lanes.iter().any(|b| *b);
                }
                self.online.selected = None;
            }
            "online-options-download" => {
                self.online.download = value == "yes";
                self.online.options.clear();
            }
            "online-download-play" => self.online.download_play = value == "yes",
            "online-check-qualities" => {
                if let (Some(i), Some(p)) = (self.online.selected, self.online_provider()) {
                    let song = self.online.songs[i].clone();
                    self.online.busy = true;
                    self.online_job(1, Job::Qualities(p, song, self.online.download), true);
                }
            }
            "online-quality" => {
                if let (Some(i), Some(q)) = (self.online.selected, index)
                    && self.online.options.get(q).is_none_or(|(ok, _)| *ok)
                {
                    self.online_play(i, Some(q), self.online.download);
                }
            }
            "online-lyrics" => {
                if !self.settings.online.lyrics_enabled {
                    self.status = "请先开启联网歌词".into();
                    return true;
                }
                self.online.lyric_target = self
                    .queue
                    .get(self.selected_id as usize)
                    .cloned()
                    .map(|source| (source, self.asset_revision));
                if self.online.lyric_target.is_none() {
                    self.status = "请先播放歌曲".into();
                    return true;
                }
                self.online.lyric_mode = true;
                self.online.provider = 0;
                self.online.source = 0;
                self.online.query = self.assets.title.clone();
                self.online_reset_search();
                self.visit_page(6);
                self.panel_open = false;
            }
            "online-music-mode" => {
                self.online.lyric_mode = false;
                self.online.provider = 0;
                self.online.source = 0;
                self.online.lyric_target = None;
                self.online_reset_search();
            }
            _ => return false,
        }
        true
    }
    fn online_directory_mut(&mut self, i: usize) -> Option<&mut std::path::PathBuf> {
        let s = &mut self.settings.online;
        match i {
            0 => Some(&mut s.api_directory),
            1 => Some(&mut s.download_directory),
            2 => Some(&mut s.lyric_directory),
            3 => Some(&mut s.artwork_directory),
            4 => Some(&mut s.stream_directory),
            _ => None,
        }
    }
    fn online_choose_lyrics(&mut self, i: usize) {
        if !self.settings.online.lyrics_enabled {
            return;
        }
        let Some(song) = self.online.songs.get(i).cloned() else {
            return;
        };
        let Some(p) = self.online_provider() else {
            return;
        };
        self.online.busy = true;
        self.online_job(
            3,
            Job::Lyrics(
                p,
                song,
                online::Directories::from_settings(&self.settings.online).lyric,
                self.online.playing.as_ref().map(Song::key),
            ),
            true,
        );
    }
    fn online_reset_search(&mut self) {
        if let Some(s) = &self.online.service {
            self.online.ids[0] = s.cancel(0);
            self.online.ids[2] = s.cancel(2);
            if self.online.lyric_mode {
                self.online.ids[3] = s.cancel(3);
                self.online.busy_lanes[3] = false;
            }
        }
        self.online.busy_lanes[0] = false;
        self.online.busy_lanes[2] = false;
        self.online.songs.clear();
        self.online.result_bytes = 0;
        self.online.result_limited = false;
        self.online.rows = Default::default();
        self.online.revision += 1;
        self.online.busy = self.online.busy_lanes.iter().any(|b| *b);
        self.online.more = false;
        self.online.selected = None;
        self.online.art_pending.clear();
        self.online.art_done.clear();
        self.online.covers.clear();
        self.online.message.clear();
    }
    fn online_cancel(&mut self) {
        if let Some(s) = &self.online.service {
            for i in 0..4 {
                self.online.ids[i] = s.cancel(i);
            }
        }
        self.online.busy_lanes = [false; 4];
        self.online.busy = false;
        self.online.art_pending.clear();
    }
    pub(super) fn online_stop_proxy(&mut self) {
        if let Some(p) = self.online.proxy.take() {
            p.stop();
            self.online.retired.push(p);
        }
    }
    pub(super) fn online_local_open(&mut self) {
        self.online_stop_proxy();
        self.online.playing = None;
        self.online.streaming = false;
        self.online.playing_source = None;
        if let Some(s) = &self.online.service {
            self.online.ids[1] = s.cancel(1);
            self.online.ids[3] = s.cancel(3);
        }
        self.online.busy_lanes[1] = false;
        self.online.busy_lanes[3] = false;
        self.online.busy = self.online.busy_lanes.iter().any(|b| *b);
        self.online.lyric_target = None;
    }
    fn online_start_audio(
        &mut self,
        song: Song,
        source: MediaSource,
        proxy: Option<online::proxy::Proxy>,
    ) {
        self.online_stop_proxy();
        self.online.proxy = proxy;
        self.online.playing = Some(song.clone());
        self.online.playing_source = Some(source.clone());
        self.online.active_id = self.online.ids[1];
        self.online.streaming = !self.online.download;
        self.queue = vec![source.clone()];
        self.queue_version += 1;
        self.selected_id = 0;
        self.video_session = false;
        self.video_resume = None;
        self.cancel_hold();
        self.send(PlaybackCommand::SetAbLoop(None));
        self.ab_start = None;
        self.ab_active = false;
        self.send(PlaybackCommand::Pause);
        self.read_assets(&source);
        // Network metadata takes precedence; asynchronous artwork/lyrics arrive later.
        self.assets.title = song.title;
        self.assets.artist = song.artist;
        self.assets.album = song.album;
        self.apply_eq();
        self.preview = false;
        self.pending_load = true;
        self.auto_page = false;
        self.send(PlaybackCommand::Load {
            source,
            decode: self.settings.global.clone(),
            resume: None,
            video: false,
        });
    }
    pub(super) fn poll_online(&mut self) {
        self.online.retired.retain(|p| !p.is_finished());
        if self.online.needs_reload {
            self.online_reload();
        }
        let replies: Vec<_> = self
            .online
            .service
            .as_ref()
            .map(|s| s.replies.try_iter().collect())
            .unwrap_or_default();
        for reply in replies {
            if reply.id != self.online.ids[reply.lane] {
                continue;
            }
            let terminal = match &reply.event {
                Event::Config(_)
                | Event::Songs(_)
                | Event::Qualities(_)
                | Event::Playing(..)
                | Event::Downloaded(..) => true,
                Event::Lyrics(..) => reply.lane == 3,
                _ => false,
            };
            if terminal {
                self.online.busy_lanes[reply.lane] = false;
                self.online.busy = self.online.busy_lanes.iter().any(|b| *b);
            }
            match reply.event {
                Event::Config(result) => match result {
                    Ok(c) => {
                        self.online.config = Some(c);
                        self.online.provider = 0;
                        self.online.source = 0;
                        self.online.message = "API配置已读取".into();
                        self.online_reset_search();
                    }
                    Err(e) => self.online.message = format!("配置未替换：{e}"),
                },
                Event::Songs(result) => match result {
                    Ok(songs) => {
                        let previous_count = self.online.songs.len();
                        self.online.more = songs.len()
                            == self.online_provider().map(|p| p.page_size).unwrap_or(20);
                        for song in songs {
                            if self.online.songs.len() >= 2000 {
                                self.online.more = false;
                                self.online.result_limited = true;
                                break;
                            }
                            if !self.online.songs.iter().any(|s| s.key() == song.key()) {
                                let bytes = [
                                    &song.provider,
                                    &song.source,
                                    &song.id,
                                    &song.title,
                                    &song.artist,
                                    &song.album,
                                    &song.lyric_id,
                                    &song.pic_id,
                                    &song.artwork_url,
                                ]
                                .iter()
                                .map(|s| s.len())
                                .sum::<usize>()
                                    + song
                                        .variables
                                        .iter()
                                        .map(|(k, v)| k.len() + v.len())
                                        .sum::<usize>()
                                    + song
                                        .qualities
                                        .iter()
                                        .flatten()
                                        .map(String::len)
                                        .sum::<usize>();
                                if self.online.result_bytes + bytes > 32 * 1024 * 1024 {
                                    self.online.more = false;
                                    self.online.result_limited = true;
                                    break;
                                }
                                self.online.result_bytes += bytes;
                                self.online.songs.push(song);
                            }
                        }
                        // An API may ignore the page argument and repeat a full
                        // page. Stop automatic pagination when nothing was added.
                        if self.online.page > 1 && self.online.songs.len() == previous_count {
                            self.online.more = false;
                        }
                        self.online_refresh_rows();
                        self.online.message = format!(
                            "{}个结果{}",
                            self.online.songs.len(),
                            if self.online.result_limited {
                                "；已达结果上限，请缩小关键词"
                            } else {
                                ""
                            }
                        );
                    }
                    Err(e) => self.online.message = e,
                },
                Event::Qualities(options) => {
                    self.online.options = options;
                }
                Event::Playing(song, result) => match result {
                    Ok((source, proxy, report)) => {
                        self.online_start_audio(song, source, proxy);
                        self.online.message = report;
                    }
                    Err(e) => self.online.message = e,
                },
                Event::Downloaded(song, result) => match result {
                    Ok(path) => {
                        if self.settings.online.downloads.len() >= 256
                            && let Some(key) = self.settings.online.downloads.keys().next().cloned()
                        {
                            self.settings.online.downloads.remove(&key);
                        }
                        self.settings.online.downloads.insert(
                            path.clone(),
                            player_core::online::DownloadMetadata {
                                title: song.title.chars().take(512).collect(),
                                artist: song.artist.chars().take(512).collect(),
                                album: song.album.chars().take(512).collect(),
                            },
                        );
                        self.import_library_paths(false, vec![path.clone()]);
                        self.online.message = format!("已下载：{}", path.display());
                        if self.online.download_play {
                            self.online_start_audio(song, MediaSource::Local(path), None);
                        }
                    }
                    Err(e) => self.online.message = e,
                },
                Event::DownloadComplete(path) => {
                    self.import_library_paths(false, vec![path]);
                }
                Event::Progress(n, total) => {
                    self.online.message = total
                        .map(|t| {
                            format!(
                                "下载中 {:.1}% · {:.1} MB",
                                100.0 * n as f64 / t as f64,
                                n as f64 / 1048576.0
                            )
                        })
                        .unwrap_or_else(|| format!("下载中 {:.1} MB", n as f64 / 1048576.0))
                }
                Event::Lyrics(song, result) => {
                    let manual = reply.lane == 3;

                    let matches = if manual {
                        self.online.lyric_target.as_ref().is_some_and(|(s, r)| {
                            self.queue.get(self.selected_id as usize) == Some(s)
                                && *r == self.asset_revision
                        })
                    } else {
                        self.settings.online.lyrics_enabled
                            && reply.id == self.online.active_id
                            && self
                                .online
                                .playing
                                .as_ref()
                                .is_some_and(|s| s.key() == song.key())
                    };
                    if matches && let Ok(text) = result {
                        if let Ok(lyrics) = player_core::lyrics::Lyrics::parse(&text) {
                            // Preserve manual/sidecar/embedded lyrics on automatic fetch.
                            if manual
                                || self.assets.lyrics.lines.is_empty()
                                    && self.assets.lyrics.plain.is_empty()
                            {
                                self.assets.lyrics = lyrics;
                                self.online_project_lyrics();
                            }
                            if manual {
                                if let Some(MediaSource::Local(path)) =
                                    self.queue.get(self.selected_id as usize)
                                {
                                    self.settings.audio.lyric_files.insert(
                                        path.clone(),
                                        online::Directories::from_settings(&self.settings.online)
                                            .lyric
                                            .join("selected")
                                            .join(format!(
                                                "{}.lrc",
                                                online::repository::hash(&song.key())
                                            )),
                                    );
                                    self.save();
                                }
                                self.online.message = "歌词已关联当前歌曲".into();
                                self.visit_page(4);
                            }
                        }
                    } else if manual {
                        self.online.message = "歌词未获取或当前歌曲已变化".into();
                    }
                }
                Event::Artwork(song, result, color) => {
                    self.online.art_pending.remove(&song.key());
                    self.online.art_done.insert(song.key());
                    if let Ok(Some((width, height, bytes))) = result {
                        let image = slint::Image::from_rgba8(slint::SharedPixelBuffer::<
                            slint::Rgba8Pixel,
                        >::clone_from_slice(
                            &bytes, width, height
                        ));
                        if reply.lane == 2 {
                            if self.online.covers.len() >= 32 {
                                self.online.covers.clear();
                            }
                            self.online.covers.insert(song.key(), image.clone());
                            self.online_refresh_rows();
                        }
                        if reply.lane == 1
                            && reply.id == self.online.active_id
                            && self
                                .online
                                .playing
                                .as_ref()
                                .is_some_and(|s| s.key() == song.key())
                        {
                            self.cover = image;
                            self.assets.cover_color = color;
                            self.asset_projection_revision += 1;
                        }
                    }
                }
                Event::Directory(kind, path) => {
                    if let Some(target) = self.online_directory_mut(kind) {
                        *target = path;
                        self.online.needs_reload = kind == 0;
                        self.save();
                    }
                }
                Event::DirectoryOpened(result) => {
                    self.status = result
                        .err()
                        .unwrap_or_else(|| "已打开 API 配置文件夹".into());
                    self.notice_revision += 1;
                }
            }
        }
        // A local lyric candidate sheet belongs to one media revision.
        if self.online.lyric_mode
            && self.online.lyric_target.as_ref().is_some_and(|(s, r)| {
                self.queue.get(self.selected_id as usize) != Some(s) || *r != self.asset_revision
            })
        {
            self.online.lyric_mode = false;
            self.online.provider = 0;
            self.online.source = 0;
            self.online.lyric_target = None;
            self.online_reset_search();
        }
    }
    fn online_project_lyrics(&mut self) {
        self.lyric_rows = Rc::new(
            self.assets
                .lyrics
                .lines
                .iter()
                .map(|l| (l.text.clone(), l.milliseconds as i32))
                .collect(),
        );
        self.plain_lyrics = Rc::new(self.assets.lyrics.plain.clone());
        self.asset_projection_revision += 1;
    }
    pub fn request_online_covers(&mut self, first: i32, count: i32) {
        if self.navigation.current() != 6
            || !self.settings.online.music_enabled
            || self.online.lyric_mode
        {
            return;
        }
        let dirs = online::Directories::from_settings(&self.settings.online);
        let songs: Vec<_> = self
            .online
            .songs
            .iter()
            .skip(first.max(0) as usize)
            .take(count.clamp(0, 20) as usize)
            .cloned()
            .collect();
        for song in songs {
            if self.online.art_pending.len() >= 2 {
                break;
            }
            if self.online.art_done.contains(&song.key())
                || self.online.art_pending.contains(&song.key())
            {
                continue;
            }
            if let Some(p) = self
                .online
                .config
                .as_ref()
                .and_then(|c| c.providers.iter().find(|p| p.id == song.provider))
                .cloned()
            {
                let key = song.key();
                if self.online_job(2, Job::Artwork(p, song, dirs.artwork.clone()), false) {
                    self.online.art_pending.insert(key);
                }
            }
        }
    }
    pub(super) fn online_view(&self) -> OnlineViewModel {
        let providers = self.online_providers().map(|p| p.name.clone()).collect();
        let p = self.online_provider();
        let dirs = online::Directories::from_settings(&self.settings.online);
        OnlineViewModel {
            revision: self.online.revision,
            rows: self.online.rows.clone(),
            providers,
            sources: p
                .as_ref()
                .map(|p| p.sources.values().cloned().collect())
                .unwrap_or_default(),
            provider: self.online.provider as i32,
            source: self.online.source as i32,
            query: self.online.query.clone(),
            history: self.settings.online.history.clone(),
            keep_history: self.settings.online.keep_history,
            music_enabled: self.settings.online.music_enabled,
            lyrics_enabled: self.settings.online.lyrics_enabled,
            busy: self.online.busy,
            message: self.online.message.clone(),
            more: self.online.more,
            lyric_mode: self.online.lyric_mode,
            selected: self.online.selected.map(|i| i as i32).unwrap_or(-1),
            qualities: p
                .as_ref()
                .map(|p| {
                    p.qualities
                        .iter()
                        .enumerate()
                        .map(|(i, q)| {
                            format!(
                                "{} · {}",
                                q.name,
                                self.online
                                    .options
                                    .get(i)
                                    .map(|(_, s)| s.as_str())
                                    .unwrap_or("请求音质；尚未检查")
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            quality_enabled: p
                .as_ref()
                .map(|p| {
                    p.qualities
                        .iter()
                        .enumerate()
                        .map(|(i, q)| {
                            self.online
                                .selected
                                .and_then(|n| self.online.songs.get(n))
                                .is_some_and(|s| eligible(s, q))
                                && self.online.options.get(i).is_none_or(|(ok, _)| *ok)
                        })
                        .collect()
                })
                .unwrap_or_default(),
            download: self.online.download,
            download_play: self.online.download_play,
            directories: [
                dirs.api,
                dirs.download,
                dirs.lyric,
                dirs.artwork,
                dirs.stream,
            ]
            .iter()
            .map(|p| p.display().to_string())
            .collect(),
        }
    }
}
