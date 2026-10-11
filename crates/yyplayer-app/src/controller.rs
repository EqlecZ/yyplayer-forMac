use crate::{
    demo,
    services::{self, DialogReply, DialogRequest, Dialogs, Persistence},
};
use player_core::settings::{DecodeMode, DecodeOptions, Settings};
use player_core::shortcuts::{HoldEffect, HoldGesture, ShortcutSettings};
use player_core::{MediaSource, PlaybackCommand, PlaybackEngine, PlaybackPhase};
use player_mpv::MpvEngine;
use player_platform::PlatformInfo;
use player_ui::{
    UiAction,
    view_model::{MediaPreview, ShellViewModel},
};
use std::path::PathBuf;
use std::time::{Duration, Instant};
#[path = "audio_controller.rs"]
mod audio_controller;
#[path = "background_controller.rs"]
mod background_controller;
#[path = "backgrounds.rs"]
mod backgrounds;
#[path = "font_controller.rs"]
mod font_controller;
#[path = "font_service.rs"]
mod font_service;
#[path = "library_controller.rs"]
mod library_controller;
#[path = "library_service.rs"]
mod library_service;
#[path = "music_assets.rs"]
mod music_assets;
#[path = "navigation.rs"]
mod navigation;
#[path = "online/mod.rs"]
mod online;
#[path = "online_controller.rs"]
mod online_controller;
#[path = "thumbnails.rs"]
mod thumbnails;
#[path = "video_controller.rs"]
mod video_controller;

pub struct AppController {
    online: online_controller::State,
    engine: MpvEngine,
    platform: PlatformInfo,
    navigation: navigation::History,
    selected_id: i32,
    search: String,
    video_search: String,
    video_session: bool,
    video_resume: Option<(MediaSource, Duration)>,
    video_service: Option<library_service::Service>,
    video_pending: Option<library_service::Request>,
    video_songs: Vec<player_core::library::Song>,
    video_rows: std::rc::Rc<Vec<MediaPreview>>,
    video_revision: u64,
    video_scan: u64,
    video_folder: i32,
    video_busy: bool,
    video_message: String,
    favorite: bool,
    status: String,
    preview: bool,
    settings: Settings,
    shortcut_draft: ShortcutSettings,
    decode_draft: DecodeOptions,
    decode_scope: i32,
    settings_revision: u64,
    save_revision: u64,
    notice_revision: u64,
    history_dirty: Option<Instant>,
    persistence: Option<Persistence>,
    dialogs: Option<Dialogs>,
    queue: Vec<MediaSource>,
    queue_version: u64,
    queue_projection: std::cell::RefCell<QueueProjection>,
    pending_load: bool,
    auto_page: bool,
    hold: HoldGesture,
    base_speed: f64,
    started: Instant,
    panel_open: bool,
    window_mode: i32,
    window_request: Option<i32>,
    prior_window_mode: i32,
    ab_start: Option<f64>,
    ab_active: bool,
    last_generation: u64,
    last_phase: PlaybackPhase,
    settings_dirty: Option<Instant>,
    initial_audio: bool,
    eq_draft: player_core::audio::EqPreset,
    eq_scope: i32,
    preset_index: i32,
    assets_service: Option<music_assets::Service>,
    asset_revision: u64,
    assets: music_assets::Assets,
    cover: slint::Image,
    expanded: bool,
    asset_projection_revision: u64,
    lyric_rows: std::rc::Rc<Vec<(String, i32)>>,
    plain_lyrics: std::rc::Rc<String>,
    pending_asset: Option<music_assets::Request>,
    library_service: Option<library_service::Service>,
    thumbnails: Option<thumbnails::Thumbnails>,
    library_pending: Option<library_service::Request>,
    library_songs: Vec<player_core::library::Song>,
    library_rows: std::rc::Rc<Vec<MediaPreview>>,
    library_revision: u64,
    library_scan: u64,
    library_folder: i32,
    library_busy: bool,
    library_message: String,
    appearance_observer: Option<player_platform::appearance::Observer>,
    backgrounds: Option<backgrounds::Backgrounds>,
    font_task: Option<font_service::Task>,
    font_names: std::rc::Rc<Vec<String>>,
    font_revision: u64,
    font_message: String,
}
impl AppController {
    pub fn new(engine: MpvEngine, platform: PlatformInfo) -> Self {
        let settings = Settings::default();
        Self {
            online: Default::default(),
            engine,
            platform,
            navigation: Default::default(),
            selected_id: 0,
            search: String::new(),
            video_search: String::new(),
            video_session: false,
            video_resume: None,
            video_service: None,
            video_pending: None,
            video_songs: vec![],
            video_rows: Default::default(),
            video_revision: 1,
            video_scan: 0,
            video_folder: 0,
            video_busy: false,
            video_message: "添加目录或视频，建立你的私人剧场".into(),
            favorite: false,
            status: String::new(),
            preview: true,
            shortcut_draft: settings.shortcuts.clone(),
            decode_draft: settings.global.clone(),
            settings,
            decode_scope: 0,
            settings_revision: 1,
            save_revision: 0,
            notice_revision: 0,
            history_dirty: None,
            persistence: None,
            dialogs: None,
            queue: vec![],
            queue_version: 1,
            queue_projection: Default::default(),
            pending_load: false,
            auto_page: false,
            hold: HoldGesture::default(),
            base_speed: 1.0,
            started: Instant::now(),
            panel_open: false,
            window_mode: 0,
            window_request: None,
            prior_window_mode: 0,
            ab_start: None,
            ab_active: false,
            last_generation: 0,
            last_phase: PlaybackPhase::Idle,
            settings_dirty: None,
            initial_audio: false,
            eq_draft: Default::default(),
            eq_scope: 0,
            preset_index: -1,
            assets_service: None,
            asset_revision: 0,
            assets: Default::default(),
            cover: Default::default(),
            expanded: false,
            asset_projection_revision: 0,
            lyric_rows: Default::default(),
            plain_lyrics: Default::default(),
            pending_asset: None,
            library_service: None,
            thumbnails: None,
            library_pending: None,
            library_songs: vec![],
            library_rows: Default::default(),
            library_revision: 1,
            library_scan: 0,
            library_folder: 0,
            library_busy: false,
            library_message: "添加目录，建立你的音乐空间".into(),
            appearance_observer: None,
            backgrounds: None,
            font_task: None,
            font_names: std::rc::Rc::new(vec![String::new()]),
            font_revision: 1,
            font_message: "正在读取系统字体…".into(),
        }
    }
    pub fn live(engine: MpvEngine, platform: PlatformInfo) -> Self {
        let path = services::settings_path();
        let (settings, warning) = services::load(&path);
        let mut controller = Self::new(engine, platform);
        controller.preview = false;
        controller.font_task = Some(font_service::Task::start());
        controller.thumbnails = Some(thumbnails::Thumbnails::start());
        controller.status = warning;
        controller.shortcut_draft = settings.shortcuts.clone();
        controller.decode_draft = settings.global.clone();
        controller.settings = settings;
        controller.online.service = Some(online::Service::start());
        if let Some(service) = &controller.online.service {
            service.set_permissions(
                controller.settings.online.music_enabled,
                controller.settings.online.lyrics_enabled,
            );
        }
        controller.online.download_play = true;
        controller.online_reload();
        controller.backgrounds = Some(backgrounds::Backgrounds::start());
        for kind in 0..2 {
            controller.request_background(kind);
        }
        controller.library_service = Some(library_service::Service::start(
            path.with_extension("library.json"),
            controller.settings.library.clone(),
        ));
        controller.video_service = Some(library_service::Service::start_video(
            path.with_extension("videos.json"),
            controller.settings.video_library.clone(),
        ));
        if !controller.settings.video_library.roots.is_empty()
            || !controller.settings.video_library.files.is_empty()
        {
            controller.scan_video();
        }
        controller.appearance_observer = Some(player_platform::appearance::Observer::start());
        if !controller.settings.library.roots.is_empty()
            || !controller.settings.library.files.is_empty()
        {
            controller.scan_library();
        }
        controller.persistence = Some(Persistence::start(path));
        controller.dialogs = Some(Dialogs::start());
        controller.assets_service = Some(music_assets::Service::start());
        controller.eq_draft = controller.settings.audio.global_eq.clone();
        controller
    }
    pub fn dispatch(&mut self, action: UiAction) {
        match action {
            UiAction::Navigate(page) => {
                // Page 1 is the player session; the sidebar opens page 5 (video library).
                let page = if page == 1 { 5 } else { page.clamp(0, 6) };
                if page == 6 && self.online.lyric_mode {
                    self.online_control("online-music-mode", "");
                }
                self.visit_page(page);
            }
            UiAction::Search(query) => {
                if self.navigation.current() == 5 {
                    self.video_search = query;
                    self.refresh_video();
                } else {
                    self.search = query;
                    self.refresh_library();
                }
            }
            UiAction::SelectPreview(id) => {
                if self.preview {
                    if demo::tracks().iter().any(|item| item.id == id) {
                        self.selected_id = id;
                        self.favorite = false;
                    }
                } else if id >= 0 && (id as usize) < self.queue.len() {
                    self.load_index(id);
                }
            }
            UiAction::SetVolume(volume) => {
                if volume.is_finite() {
                    self.settings.volume = volume.clamp(0.0, 100.0);
                    self.settings_dirty = Some(Instant::now());
                    self.send(PlaybackCommand::SetVolume(self.settings.volume));
                }
            }
            UiAction::ToggleFavorite => self.favorite = !self.favorite,
            UiAction::OpenFile => self.dialog(DialogRequest::Open),
            UiAction::RequestPlayback => self.toggle_pause(),
            UiAction::Previous => self.select_adjacent(-1),
            UiAction::Next => self.select_adjacent(1),
            UiAction::Control(action, value) => self.control(&action, &value),
            UiAction::ShortcutEdited(index, value) => {
                let field = match index {
                    0 => &mut self.shortcut_draft.toggle_pause,
                    1 => &mut self.shortcut_draft.seek_back,
                    2 => &mut self.shortcut_draft.seek_forward,
                    3 => &mut self.shortcut_draft.volume_up,
                    4 => &mut self.shortcut_draft.volume_down,
                    5 => &mut self.shortcut_draft.fullscreen,
                    6 => &mut self.shortcut_draft.mute,
                    7 => &mut self.shortcut_draft.info,
                    8 => &mut self.shortcut_draft.open,
                    9 => &mut self.shortcut_draft.frame_step,
                    _ => return,
                };
                *field = value;
            }
        }
    }
    fn visit_page(&mut self, page: i32) {
        let previous = self.navigation.current();
        if self.navigation.visit(page) {
            self.page_changed(previous);
        }
    }
    fn go_back(&mut self) {
        let previous = self.navigation.current();
        if !self.navigation.back() {
            return;
        }
        self.page_changed(previous);
        if self.navigation.current() == 1 && !self.video_session {
            let source = self.queue.get(self.selected_id as usize);
            let snapshot = self.engine.snapshot();
            let resumable = self
                .video_resume
                .as_ref()
                .is_some_and(|(saved, _)| Some(saved) == source)
                || snapshot.video && snapshot.source.as_ref() == source && source.is_some();
            if resumable {
                // The popped page is already current. Loading it cannot push
                // Settings back onto the history or create a return loop.
                self.load_index(self.selected_id);
            }
        }
    }
    fn page_changed(&mut self, previous: i32) {
        self.cancel_hold();
        if previous == 6 && self.online.busy {
            if let Some(service) = &self.online.service {
                self.online.ids[0] = service.cancel(0);
                self.online.ids[1] = service.cancel(1);
                self.online.ids[3] = service.cancel(3);
            }
            self.online.busy_lanes = [false; 4];
            self.online.busy = false;
        }
        if previous == 1 {
            if self.video_session {
                self.suspend_video();
            } else {
                self.leave_fullscreen();
            }
        } else if previous == 4 {
            self.leave_fullscreen();
        }
        self.auto_page = false;
        self.expanded = self.navigation.current() == 4;
        self.panel_open = false;
        self.settings_revision += 1;
    }
    fn send(&mut self, command: PlaybackCommand) {
        if let Err(error) = self.engine.submit(command) {
            self.status = error.to_string();
        }
    }
    fn dialog(&mut self, request: DialogRequest) {
        self.cancel_hold();
        if let Some(dialogs) = &self.dialogs {
            if let Err(error) = dialogs.commands.try_send(request) {
                self.status = format!("文件选择暂时不可用：{error}");
            }
        } else {
            self.status = "界面截图模式不打开文件".into();
        }
    }
    pub fn request_library_covers(&mut self, first: i32, count: i32) {
        if self.navigation.current() != 0 || self.preview {
            return;
        }
        if let Some(thumbnails) = &mut self.thumbnails {
            for row in self
                .library_rows
                .iter()
                .skip(first.max(0) as usize)
                .take(count.clamp(0, 64) as usize)
            {
                if let Some(song) = self.library_songs.get(row.id as usize) {
                    thumbnails.request(&song.path);
                }
            }
        }
    }
    pub fn prepare_paths(&mut self, paths: Vec<PathBuf>) {
        self.dialog(DialogRequest::Paths(paths));
    }
    pub fn prepare_video_paths(&mut self, folder: bool, paths: Vec<PathBuf>) {
        self.video_request(library_service::Request::Paths(folder, paths));
    }
    pub fn open_paths(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        self.queue = paths.into_iter().map(MediaSource::Local).collect();
        self.queue_version += 1;
        self.preview = false;
        self.load_index(0);
    }
    fn load_index(&mut self, index: i32) {
        let Some(source) = self.queue.get(index as usize).cloned() else {
            return;
        };
        if self.online.playing_source.as_ref() == Some(&source) {
            if self.online.streaming
                && (!self.settings.online.music_enabled
                    || matches!(&source, MediaSource::Url(_)) && self.online.proxy.is_none())
            {
                self.status = "请开启联网音乐并从搜索结果重新播放".into();
                return;
            }
            self.send(PlaybackCommand::Load {
                source,
                decode: self.settings.global.clone(),
                resume: None,
                video: false,
            });
            return;
        }
        self.online_local_open();
        let video = !matches!(&source, MediaSource::Local(p) if player_core::library::is_audio(p));
        let restore = self
            .video_resume
            .take()
            .filter(|(saved, _)| video && saved == &source)
            .map(|(_, position)| (position.as_secs_f64(), false));
        self.video_session = video;
        self.cancel_hold();
        self.send(PlaybackCommand::SetAbLoop(None));
        self.ab_start = None;
        self.ab_active = false;
        self.selected_id = index;
        // Startup paths arrive before the first UI tick. Queue saved settings
        // first so an unavailable device cannot briefly play through `auto`.
        #[cfg(target_os = "linux")]
        if !self.initial_audio {
            self.initial_audio = true;
            self.send(PlaybackCommand::SetVolume(self.settings.volume));
            self.apply_output();
            self.apply_subtitle_font();
        }
        self.send(PlaybackCommand::Pause);
        self.read_assets(&source);
        self.apply_eq();
        self.visit_page(if video {
            1
        } else if self.expanded {
            4
        } else {
            0
        });
        self.pending_load = true;
        self.auto_page = true;
        self.status = String::new();
        self.favorite = false;
        let decode = match &source {
            MediaSource::Local(path) => {
                self.settings.recent.retain(|recent| recent != path);
                self.settings.recent.insert(0, path.clone());
                self.settings.recent.truncate(30);
                self.history_dirty = Some(Instant::now());
                self.settings.resolve(path).0
            }
            _ => self.settings.global.clone(),
        };
        self.decode_draft = decode.clone();
        self.decode_scope = 2;
        self.settings_revision += 1;
        self.send(PlaybackCommand::Load {
            source,
            decode,
            resume: restore,
            video,
        });
    }
    fn select_adjacent(&mut self, offset: i32) {
        if self.preview {
            self.selected_id = (self.selected_id + offset).rem_euclid(demo::tracks().len() as i32);
        } else if !self.queue.is_empty() {
            let index = self.selected_id + offset;
            if index >= 0 && (index as usize) < self.queue.len() {
                self.load_index(index);
            }
        }
    }
    fn toggle_pause(&mut self) {
        if !self.engine.snapshot().ready {
            self.status = "播放内核尚未就绪，请检查运行时或错误提示".into();
            return;
        }
        if self.engine.snapshot().phase == PlaybackPhase::Ended {
            self.send(PlaybackCommand::Seek(Duration::ZERO));
            self.send(PlaybackCommand::Resume);
        } else if self.engine.snapshot().phase == PlaybackPhase::Playing {
            self.send(PlaybackCommand::Pause);
        } else if matches!(
            self.engine.snapshot().phase,
            PlaybackPhase::Paused | PlaybackPhase::Loading
        ) {
            self.send(PlaybackCommand::Resume);
        } else if !self.queue.is_empty() {
            self.load_index(self.selected_id);
        } else {
            self.dialog(DialogRequest::Open);
        }
    }
    fn save(&mut self) {
        self.save_with_notice(true);
    }
    fn save_with_notice(&mut self, notify: bool) {
        self.settings_dirty = None;
        self.history_dirty = None;
        self.save_revision += 1;
        if let Some(persistence) = &self.persistence
            && let Err(error) =
                persistence
                    .commands
                    .try_send((self.save_revision, self.settings.clone(), notify))
        {
            self.status = format!("设置保存未提交：{error}");
            self.notice_revision += 1;
            if notify {
                self.settings_dirty = Some(Instant::now());
            } else {
                self.history_dirty = Some(Instant::now());
            }
        }
    }
    fn reload(&mut self) {
        self.cancel_hold();
        let Some(source) = self.queue.get(self.selected_id as usize).cloned() else {
            return;
        };
        let decode = match &source {
            MediaSource::Local(path) => self.settings.resolve(path).0,
            _ => self.settings.global.clone(),
        };
        let snapshot = self.engine.snapshot();
        let restore = snapshot.position.map(|position| {
            (
                position.as_secs_f64(),
                snapshot.phase != PlaybackPhase::Playing,
            )
        });
        self.send(PlaybackCommand::Load {
            source,
            decode,
            resume: restore,
            video: self.video_session,
        });
        self.pending_load = true;
        self.auto_page = false;
    }
    fn scope_options(&mut self, scope: i32) {
        self.decode_scope = scope.clamp(0, 2);
        self.decode_draft = match self.queue.get(self.selected_id as usize) {
            Some(MediaSource::Local(path)) if scope == 2 => self
                .settings
                .files
                .get(path)
                .cloned()
                .unwrap_or_else(|| self.settings.resolve(path).0),
            Some(MediaSource::Local(path)) if scope == 1 => path
                .parent()
                .and_then(|parent| self.settings.folders.get(parent))
                .cloned()
                .unwrap_or_else(|| self.settings.resolve(path).0),
            _ => self.settings.global.clone(),
        };
        self.settings_revision += 1;
    }
    fn commit_decode(&mut self, clear: bool) {
        if let Err(error) = self.decode_draft.validate() {
            self.status = error;
            return;
        }
        let path = match self.queue.get(self.selected_id as usize) {
            Some(MediaSource::Local(path)) => Some(path.clone()),
            _ => None,
        };
        match self.decode_scope {
            0 => {
                self.settings.global = if clear {
                    DecodeOptions::default()
                } else {
                    self.decode_draft.clone()
                }
            }
            1 => {
                if let Some(parent) = path.as_ref().and_then(|path| path.parent()) {
                    if clear {
                        self.settings.folders.remove(parent);
                    } else {
                        self.settings
                            .folders
                            .insert(parent.into(), self.decode_draft.clone());
                    }
                } else {
                    self.status = "文件夹规则需要本地文件".into();
                    return;
                }
            }
            2 => {
                if let Some(path) = path {
                    if clear {
                        self.settings.files.remove(&path);
                    } else {
                        self.settings.files.insert(path, self.decode_draft.clone());
                    }
                } else {
                    self.status = "单文件规则需要本地文件；URL 使用全局规则".into();
                    return;
                }
            }
            _ => return,
        }
        self.scope_options(self.decode_scope);
        self.save();
        self.reload();
    }
    fn control(&mut self, action: &str, value: &str) {
        if self.online_control(action, value) {
            return;
        }
        if self.font_control(action, value) {
            return;
        }
        if self.video_control(action, value) {
            return;
        }
        if self.library_control(action, value) {
            return;
        }
        if self.background_control(action, value) {
            return;
        }
        if self.audio_control(action, value) {
            return;
        }
        let number = value.parse::<f64>().ok().filter(|value| value.is_finite());
        match action {
            "demo-album" if self.preview => {
                if let Some(index) = number {
                    self.dispatch(UiAction::SelectPreview(index as i32));
                }
            }
            "stop" => {
                if self.online.streaming {
                    self.queue.clear();
                    self.queue_version += 1;
                    self.selected_id = 0;
                }
                self.online_local_open();
                self.cancel_hold();
                self.pending_load = false;
                self.auto_page = false;
                self.video_session = false;
                self.video_resume = None;
                self.send(PlaybackCommand::Stop);
                if self.navigation.current() == 1 {
                    self.visit_page(5);
                    self.leave_fullscreen();
                }
            }
            "queue" => {
                if let Some(index) = number
                    && index >= 0.0
                    && (index as usize) < self.queue.len()
                {
                    self.load_index(index as i32);
                }
            }
            "open-recent" => {
                if let Some(path) = number
                    .and_then(|index| self.settings.recent.get(index as usize))
                    .cloned()
                {
                    self.prepare_paths(vec![path]);
                }
            }
            "back" => self.send(PlaybackCommand::SeekRelative(
                -self.settings.shortcuts.seek_seconds,
            )),
            "forward" => self.send(PlaybackCommand::SeekRelative(
                self.settings.shortcuts.seek_seconds,
            )),
            "seek-percent" => {
                if let (Some(percent), Some(duration)) = (number, self.engine.snapshot().duration) {
                    self.send(PlaybackCommand::Seek(Duration::from_secs_f64(
                        duration.as_secs_f64() * percent.clamp(0.0, 100.0) / 100.0,
                    )));
                }
            }
            "speed" => {
                if let Ok(speed) = value.trim_end_matches('x').parse::<f64>()
                    && speed.is_finite()
                    && (0.1..=8.0).contains(&speed)
                {
                    self.cancel_hold();
                    self.base_speed = speed;
                    self.send(PlaybackCommand::SetSpeed(speed));
                }
            }
            "mute" => self.send(PlaybackCommand::SetMute(!self.engine.snapshot().muted)),
            "device" => {
                if let Some(device) =
                    number.and_then(|index| self.engine.snapshot().devices.get(index as usize))
                {
                    let id = device.id.clone();
                    self.settings.device = id.clone();
                    self.apply_output();
                    self.apply_eq();
                    self.refresh_eq_draft();
                    self.save();
                }
            }
            "audio-track" | "subtitle-track" => {
                if let Some(index) = number {
                    let kind = if action == "audio-track" {
                        "audio"
                    } else {
                        "sub"
                    };
                    let id = if index == 0.0 {
                        "no".into()
                    } else {
                        self.engine
                            .snapshot()
                            .tracks
                            .iter()
                            .filter(|track| track.kind == kind)
                            .nth(index as usize - 1)
                            .map(|track| track.id.to_string())
                            .unwrap_or("no".into())
                    };
                    self.send(PlaybackCommand::SetTrack {
                        kind: if kind == "audio" { "aid" } else { "sid" }.into(),
                        id,
                    });
                }
            }
            "chapter" => {
                if let Some(chapter) =
                    number.and_then(|index| self.engine.snapshot().chapters.get(index as usize))
                {
                    self.send(PlaybackCommand::Seek(Duration::from_secs_f64(
                        chapter.seconds.max(0.0),
                    )));
                }
            }
            "subtitle-file" => self.dialog(DialogRequest::Subtitle),
            "subtitle-delay" => {
                if let Some(delay) = number {
                    self.send(PlaybackCommand::SetSubtitleDelay(delay));
                }
            }
            "audio-delay" => {
                if let Some(delay) = number {
                    self.send(PlaybackCommand::SetAudioDelay(delay));
                }
            }
            "frame" => self.send(PlaybackCommand::FrameStep(false)),
            "frame-back" => self.send(PlaybackCommand::FrameStep(true)),
            "screenshot" => self.dialog(DialogRequest::Screenshot),
            "loop" => self.send(PlaybackCommand::SetLoop(value == "yes")),
            "aspect" => {
                let aspect = match value {
                    "1" => "16:9",
                    "2" => "4:3",
                    "3" => "2.35",
                    _ => "-1",
                };
                self.send(PlaybackCommand::SetAspect(aspect.into()));
            }
            "ab-loop" => {
                if self.ab_active {
                    self.send(PlaybackCommand::SetAbLoop(None));
                    self.ab_active = false;
                    self.ab_start = None;
                    self.status = "A-B 循环已清除".into();
                } else if let Some(position) = self.engine.snapshot().position {
                    let position = position.as_secs_f64();
                    if let Some(start) = self.ab_start {
                        if position > start {
                            self.send(PlaybackCommand::SetAbLoop(Some((start, position))));
                            self.ab_active = true;
                            self.status = format!("A-B 循环：{start:.1}–{position:.1}s");
                        } else {
                            self.status = "B 点必须晚于 A 点".into();
                        }
                    } else {
                        self.ab_start = Some(position);
                        self.status = format!("A 点：{position:.1}s；再次点击设置 B 点");
                    }
                }
            }
            "panel" | "info" => {
                self.cancel_hold();
                self.panel_open = !self.panel_open;
                self.settings_revision += 1;
            }
            "settings" => {
                self.visit_page(3);
            }
            "page-back" => self.go_back(),
            "video" => {
                if self.video_resume.is_some()
                    || (self.engine.snapshot().source.is_some() && self.engine.snapshot().video)
                {
                    self.load_index(self.selected_id);
                    self.panel_open = false;
                } else {
                    self.dispatch(UiAction::Navigate(5));
                }
            }
            "window-mode" => {
                if let Some(mode) = number {
                    self.set_window_mode(mode as i32);
                }
            }
            "fullscreen" => self.set_window_mode(if self.window_mode == 2 {
                self.prior_window_mode
            } else {
                2
            }),
            "escape" => {
                self.cancel_hold();
                if self.window_mode == 2 {
                    self.set_window_mode(self.prior_window_mode);
                } else {
                    self.panel_open = false;
                }
            }
            "wheel-volume" => {
                if let Some(delta) = number {
                    self.adjust_volume(if delta > 0.0 {
                        self.settings.shortcuts.volume_step
                    } else {
                        -self.settings.shortcuts.volume_step
                    });
                }
            }
            "decode-scope" => {
                if let Some(scope) = number {
                    self.scope_options(scope as i32);
                }
            }
            "decode-mode" => {
                if let Some(mode) = number {
                    self.decode_draft.mode = match mode as i32 {
                        1 => DecodeMode::Software,
                        2 => DecodeMode::HardwarePreferred,
                        _ => DecodeMode::Auto,
                    };
                }
            }
            "decode-threads" => {
                if let Some(threads) = number {
                    self.decode_draft.threads = threads.clamp(0.0, 32.0) as u8;
                }
            }
            "deinterlace" => self.decode_draft.deinterlace = value == "yes",
            "deband" => self.decode_draft.deband = value == "yes",
            "save-decode" => self.commit_decode(false),
            "clear-decode" => self.commit_decode(true),
            "seek-step" => {
                if let Some(step) = number {
                    self.shortcut_draft.seek_seconds = step;
                }
            }
            "volume-step" => {
                if let Some(step) = number {
                    self.shortcut_draft.volume_step = step as f32;
                }
            }
            "hold-ms" => {
                if let Some(ms) = number {
                    self.shortcut_draft.hold_ms = ms as u64;
                }
            }
            "hold-speed" => self.shortcut_draft.hold_speed = number.unwrap_or(f64::NAN),
            "save-shortcuts" => match self.shortcut_draft.validate() {
                Ok(()) => {
                    self.cancel_hold();
                    self.settings.shortcuts = self.shortcut_draft.clone();
                    self.settings_revision += 1;
                    self.save();
                }
                Err(error) => self.status = error,
            },
            "reset-shortcuts" => {
                self.cancel_hold();
                self.shortcut_draft = ShortcutSettings::default();
                self.settings.shortcuts = self.shortcut_draft.clone();
                self.settings_revision += 1;
                self.save();
            }
            "focus-video" => {}
            _ => {}
        }
    }
    fn adjust_volume(&mut self, delta: f32) {
        self.dispatch(UiAction::SetVolume(
            (self.settings.volume + delta).clamp(0.0, 100.0),
        ));
    }
    fn leave_fullscreen(&mut self) {
        if self.window_mode == 2 {
            self.set_window_mode(self.prior_window_mode);
        }
    }
    fn suspend_video(&mut self) {
        self.cancel_hold();
        let snapshot = self.engine.snapshot();
        if snapshot.source.as_ref() == self.queue.get(self.selected_id as usize) {
            self.video_resume = snapshot.source.clone().zip(snapshot.position);
        }
        self.pending_load = false;
        self.auto_page = false;
        self.video_session = false;
        self.send(PlaybackCommand::Stop);
        self.leave_fullscreen();
    }
    pub fn render_failed(&mut self, error: &str) {
        if self.video_session && !error.is_empty() {
            self.suspend_video();
            self.status = error.into();
        }
    }
    fn set_window_mode(&mut self, mode: i32) {
        self.cancel_hold();
        let mode = mode.clamp(0, 2);
        if mode == 2 && self.window_mode != 2 && self.engine.snapshot().source.is_none() {
            return;
        }
        if mode == 2 && self.window_mode != 2 {
            self.prior_window_mode = self.window_mode;
            let page = if self.engine.snapshot().source.is_some() && !self.engine.snapshot().video {
                self.expanded = true;
                4
            } else {
                self.video_session = true;
                self.send(PlaybackCommand::SetVideoOutput(true));
                1
            };
            self.visit_page(page);
        }
        self.window_mode = mode;
        self.window_request = Some(mode);
    }
    pub fn take_window_request(&mut self) -> Option<i32> {
        self.window_request.take()
    }
    pub fn native_window_mode(&mut self, fullscreen: bool, maximized: bool) {
        self.window_mode = if fullscreen {
            2
        } else if maximized {
            1
        } else {
            0
        };
    }
    pub fn key(&mut self, chord: &str, pressed: bool, repeat: bool, blocked: bool) -> bool {
        if !pressed {
            if let Some(effect) = self.hold.release(chord) {
                self.effect(effect);
                return true;
            }
            return false;
        }
        if chord == "Alt+Left"
            && self.settings.shortcuts.action(chord).is_none()
            && self.navigation.can_go_back()
        {
            if !repeat {
                self.go_back();
            }
            return true;
        }
        if chord == "Escape" && (self.window_mode == 2 || self.panel_open) {
            if !repeat {
                self.control("escape", "");
            }
            return true;
        }
        if blocked {
            self.cancel_hold();
            return false;
        }
        if chord == "Escape" {
            if !repeat {
                self.control("escape", "");
            }
            return true;
        }
        if chord == "Enter" {
            if !repeat {
                self.control("fullscreen", "");
            }
            return true;
        }
        let Some(action) = self.settings.shortcuts.action(chord).map(str::to_owned) else {
            return false;
        };
        if action == "back" || action == "forward" {
            if !repeat && self.engine.snapshot().source.is_some() {
                self.hold.press(
                    chord.into(),
                    self.started.elapsed().as_millis() as u64,
                    if action == "back" {
                        -self.settings.shortcuts.seek_seconds
                    } else {
                        self.settings.shortcuts.seek_seconds
                    },
                    self.base_speed,
                );
            }
        } else if !repeat || action == "up" || action == "down" {
            match action.as_str() {
                "pause" => self.toggle_pause(),
                "up" => self.adjust_volume(self.settings.shortcuts.volume_step),
                "down" => self.adjust_volume(-self.settings.shortcuts.volume_step),
                "open" => self.dialog(DialogRequest::Open),
                other => self.control(other, ""),
            }
        }
        true
    }
    fn effect(&mut self, effect: HoldEffect) {
        match effect {
            HoldEffect::Seek(seconds) => self.send(PlaybackCommand::SeekRelative(seconds)),
            HoldEffect::Speed(speed) => self.send(PlaybackCommand::SetSpeed(speed)),
        }
    }
    pub fn cancel_hold(&mut self) {
        if let Some(effect) = self.hold.cancel() {
            self.effect(effect);
        }
    }
    pub fn tick(&mut self) {
        if let Some(effect) = self.hold.tick(
            self.started.elapsed().as_millis() as u64,
            self.settings.shortcuts.hold_ms,
            self.settings.shortcuts.hold_speed,
        ) {
            self.effect(effect);
        }
        let replies: Vec<_> = self
            .dialogs
            .as_ref()
            .map(|dialogs| dialogs.replies.try_iter().collect())
            .unwrap_or_default();
        for reply in replies {
            match reply {
                DialogReply::Open(paths) => {
                    if self.navigation.current() == 5 {
                        self.prepare_video_paths(false, paths);
                    } else {
                        self.open_paths(paths);
                    }
                }
                DialogReply::MediaPaths(files, folders) => {
                    let video_library =
                        self.navigation.current() == 5 || self.navigation.current() == 1;
                    if video_library {
                        self.import_video_paths(true, folders);
                    } else {
                        self.import_library_paths(true, folders);
                    }
                    if !files.is_empty() {
                        if self.navigation.current() == 5 {
                            self.prepare_video_paths(false, files);
                        } else {
                            self.open_paths(files);
                        }
                    } else {
                        self.visit_page(if video_library { 5 } else { 0 });
                    }
                }
                DialogReply::Subtitle(path) => self.send(PlaybackCommand::AddSubtitle(path)),
                DialogReply::Screenshot(path) => self.send(PlaybackCommand::Screenshot(path)),
                DialogReply::Background(kind, path) => self.set_background_file(kind, path),
                DialogReply::Error(error) => self.status = error,
            }
        }
        let replies: Vec<_> = self
            .persistence
            .as_ref()
            .map(|persistence| persistence.replies.try_iter().collect())
            .unwrap_or_default();
        for (revision, result, notify) in replies {
            if revision == self.save_revision {
                match result {
                    Ok(()) if notify => {
                        self.status = "设置已保存".into();
                        self.notice_revision += 1;
                    }
                    Err(error) => {
                        self.status = format!("设置保存失败：{error}");
                        self.notice_revision += 1;
                    }
                    _ => {}
                }
            }
        }
        let changed = self.engine.refresh();
        self.poll_assets();
        self.poll_online();
        self.poll_library();
        self.poll_video();
        if self.thumbnails.as_mut().is_some_and(|t| t.poll()) {
            self.refresh_library();
        }
        self.poll_fonts();
        if let Some(backgrounds) = &mut self.backgrounds {
            for (kind, result) in backgrounds.poll() {
                if let Err(error) = result {
                    self.status = format!(
                        "{}背景：{error}",
                        if kind == 0 { "媒体库" } else { "歌词页" }
                    );
                    self.notice_revision += 1;
                }
            }
        }
        if self.engine.snapshot().ready && !self.initial_audio {
            self.initial_audio = true;
            self.send(PlaybackCommand::SetVolume(self.settings.volume));
            self.apply_output();
            self.apply_eq();
            self.apply_subtitle_font();
        }
        if changed {
            let snapshot = self.engine.snapshot().clone();
            if !snapshot.error.is_empty()
                && (!snapshot.ready
                    || snapshot.audio_blocked
                    || !self.pending_load
                    || snapshot.source.as_ref() == self.queue.get(self.selected_id as usize))
            {
                self.status = snapshot.error.clone();
            }
            if snapshot.phase == PlaybackPhase::Error
                && self.video_session
                && (!snapshot.ready
                    || snapshot.source.as_ref() == self.queue.get(self.selected_id as usize))
            {
                self.pending_load = false;
                self.auto_page = false;
                self.video_session = false;
                self.send(PlaybackCommand::Stop);
            }
            if snapshot.generation != self.last_generation {
                self.last_generation = snapshot.generation;
            }
            if self.pending_load
                && snapshot.source.as_ref() == self.queue.get(self.selected_id as usize)
                && matches!(
                    snapshot.phase,
                    PlaybackPhase::Playing | PlaybackPhase::Paused
                )
            {
                self.pending_load = false;
                if self.auto_page {
                    if snapshot.video && !self.video_session {
                        self.video_session = true;
                        self.send(PlaybackCommand::SetVideoOutput(true));
                    } else if !snapshot.video && self.video_session {
                        self.video_session = false;
                        self.send(PlaybackCommand::SetVideoOutput(false));
                    }
                    self.visit_page(if snapshot.video {
                        1
                    } else if self.expanded {
                        4
                    } else {
                        0
                    });
                    self.auto_page = false;
                }
            }
            if snapshot.phase == PlaybackPhase::Ended
                && self.last_phase != PlaybackPhase::Ended
                && !self.pending_load
                && (!snapshot.video || self.video_session)
                && (self.selected_id as usize + 1) < self.queue.len()
            {
                self.select_adjacent(1);
            }
            self.last_phase = snapshot.phase;
        }
        if self
            .settings_dirty
            .is_some_and(|started| started.elapsed() > Duration::from_millis(800))
        {
            self.save();
        } else if self.settings_dirty.is_none()
            && self
                .history_dirty
                .is_some_and(|t| t.elapsed() > Duration::from_millis(800))
        {
            self.save_with_notice(false);
        }
    }
    pub fn view_model(&self) -> ShellViewModel {
        let snapshot = self.engine.snapshot();
        let (queue, tracks, queue_revision) = self.project_queue();
        let selected = queue.get(self.selected_id as usize);
        let mut devices: Vec<_> = snapshot
            .devices
            .iter()
            .map(|device| device.name.clone())
            .collect();
        if devices.is_empty() {
            devices.push("系统默认（内核未就绪）".into());
        }
        let device_index = snapshot
            .devices
            .iter()
            .position(|device| Some(&device.id) == snapshot.audio_output.as_ref())
            .unwrap_or(0) as i32;
        let track_list = |kind: &str| {
            let filtered: Vec<_> = snapshot
                .tracks
                .iter()
                .filter(|track| track.kind == kind)
                .collect();
            let selected = filtered
                .iter()
                .position(|track| track.selected)
                .map(|index| index + 1)
                .unwrap_or(0) as i32;
            let mut labels = vec!["关闭".into()];
            labels.extend(filtered.into_iter().map(|track| track.label.clone()));
            (labels, selected)
        };
        let (audio_tracks, audio_index) = track_list("audio");
        let (subtitle_tracks, subtitle_index) = track_list("sub");
        let speed = if snapshot.ready {
            snapshot.speed.max(1.0e-3)
        } else {
            1.0
        };
        let rates = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0];
        let mut speed_choices: Vec<_> = rates.iter().map(|rate| format!("{rate}x")).collect();
        let speed_index = rates
            .iter()
            .position(|rate| (rate - speed).abs() < 0.001)
            .unwrap_or_else(|| {
                speed_choices.push(format!("{speed:.2}x"));
                rates.len()
            });
        let (_, origin) = match self.queue.get(self.selected_id as usize) {
            Some(MediaSource::Local(path)) => self.settings.resolve(path),
            _ => (self.settings.global.clone(), "全局".into()),
        };
        let labels = [
            "播放 / 暂停",
            "快退 / 长按加速",
            "快进 / 长按加速",
            "音量增加",
            "音量减少",
            "切换全屏",
            "静音",
            "媒体信息 / 选项",
            "打开文件",
            "下一帧",
        ];
        ShellViewModel {
            online: self.online_view(),
            audio: self.audio_view(),
            library: self.library_view(),
            video_library: self.video_view(),
            video_renderer_requested: self.video_session || snapshot.video_output_enabled,
            can_resume_video: self.video_resume.is_some(),
            search_query: if self.navigation.current() == 5 {
                self.video_search.clone()
            } else {
                self.search.clone()
            },
            appearance: self.appearance_view(),
            fonts: self.font_view(),
            column_song: self.settings.library_columns.song,
            column_artist: self.settings.library_columns.artist,
            window_controls_left: matches!(
                self.platform.kind,
                player_platform::PlatformKind::MacOs
            ),
            page: self.navigation.current(),
            can_go_back: self.navigation.can_go_back(),
            selected_id: self.selected_id,
            selected_title: if !self.assets.title.is_empty() {
                self.assets.title.clone()
            } else if !snapshot.title.is_empty() {
                snapshot.title.clone()
            } else {
                selected
                    .map(|track| track.title.clone())
                    .unwrap_or_else(|| {
                        if self.navigation.current() == 0
                            || self.navigation.current() == 4
                            || self.navigation.current() == 6
                        {
                            "尚未选择曲目".into()
                        } else {
                            "视频库".into()
                        }
                    })
            },
            selected_artist: if !self.assets.artist.is_empty() {
                self.assets.artist.clone()
            } else {
                selected
                    .map(|track| track.artist.clone())
                    .unwrap_or_default()
            },
            selected_duration: if self.preview {
                selected
                    .map(|track| track.duration.clone())
                    .unwrap_or("0:00".into())
            } else {
                format_time(snapshot.duration)
            },
            selected_cover: selected.map(|track| track.cover).unwrap_or(0),
            volume_percent: if snapshot.ready {
                snapshot.volume
            } else {
                self.settings.volume
            },
            favorite: self.favorite,
            status: if self.preview {
                format!("{} · 界面预览", self.platform.name)
            } else {
                self.status.clone()
            },
            status_revision: self.notice_revision,
            tracks,
            queue,
            queue_revision,
            recent: self
                .settings
                .recent
                .iter()
                .enumerate()
                .map(|(index, path)| MediaPreview {
                    id: index as i32,
                    title: source_title(&MediaSource::Local(path.clone())),
                    artist: path
                        .parent()
                        .map(|parent| parent.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    collection: String::new(),
                    duration: String::new(),
                    cover: index as i32 % 3,
                    artwork: Default::default(),
                })
                .collect(),
            playing: snapshot.phase == PlaybackPhase::Playing,
            has_media: snapshot.source.is_some(),
            has_video: snapshot.video,
            position_text: format_time(snapshot.position),
            progress: snapshot
                .position
                .zip(snapshot.duration)
                .filter(|(_, duration)| !duration.is_zero())
                .map(|(position, duration)| {
                    (position.as_secs_f64() / duration.as_secs_f64() * 100.0).clamp(0.0, 100.0)
                        as f32
                })
                .unwrap_or(0.0),
            seekable: snapshot
                .duration
                .is_some_and(|duration| !duration.is_zero()),
            muted: snapshot.muted,
            speed_index: speed_index as i32,
            speed_choices,
            info: if snapshot.info.is_empty() {
                format!("尚未加载媒体\n{}\n{}", snapshot.error, self.status)
            } else {
                format!(
                    "{}\n{}\n{}\n{}",
                    self.assets.info, snapshot.audio_status, snapshot.audio_info, snapshot.info
                )
            },
            device_names: devices,
            device_index,
            audio_tracks,
            audio_index,
            subtitle_tracks,
            subtitle_index,
            chapters: snapshot
                .chapters
                .iter()
                .map(|chapter| chapter.title.clone())
                .collect(),
            panel_open: self.panel_open,
            fullscreen: self.window_mode == 2,
            window_mode: self.window_mode,
            decode_mode: match self.decode_draft.mode {
                DecodeMode::Auto => 0,
                DecodeMode::Software => 1,
                DecodeMode::HardwarePreferred => 2,
            },
            decode_threads: self.decode_draft.threads as i32,
            deinterlace: self.decode_draft.deinterlace,
            deband: self.decode_draft.deband,
            decode_scope: self.decode_scope,
            decode_origin: origin,
            short_bindings: self
                .shortcut_draft
                .bindings()
                .iter()
                .enumerate()
                .map(|(index, (_, binding))| (labels[index].into(), (*binding).into()))
                .collect(),
            seek_step: self.shortcut_draft.seek_seconds as i32,
            volume_step: self.shortcut_draft.volume_step as i32,
            hold_ms: self.shortcut_draft.hold_ms as i32,
            hold_speed: self.shortcut_draft.hold_speed.to_string(),
            settings_revision: self.settings_revision,
        }
    }
    fn project_queue(
        &self,
    ) -> (
        std::rc::Rc<Vec<MediaPreview>>,
        std::rc::Rc<Vec<MediaPreview>>,
        u64,
    ) {
        let snapshot = self.engine.snapshot();
        let key = (
            self.queue_version,
            self.selected_id,
            self.asset_projection_revision,
            snapshot.duration,
            self.search.clone(),
            self.preview,
        );
        let mut cache = self.queue_projection.borrow_mut();
        if cache.key.as_ref() != Some(&key) {
            let queue = if self.preview {
                demo::tracks()
            } else {
                self.queue
                    .iter()
                    .enumerate()
                    .map(|(index, source)| MediaPreview {
                        id: index as i32,
                        title: if index == self.selected_id as usize
                            && !self.assets.title.is_empty()
                        {
                            self.assets.title.clone()
                        } else {
                            source_title(source)
                        },
                        artist: if index == self.selected_id as usize {
                            self.assets.artist.clone()
                        } else {
                            String::new()
                        },
                        collection: if index == self.selected_id as usize
                            && !self.assets.album.is_empty()
                        {
                            self.assets.album.clone()
                        } else {
                            "本地媒体".into()
                        },
                        duration: if index == self.selected_id as usize {
                            format_time(snapshot.duration)
                        } else {
                            String::new()
                        },
                        cover: -1,
                        artwork: if index == self.selected_id as usize {
                            self.cover.clone()
                        } else {
                            Default::default()
                        },
                    })
                    .collect()
            };
            let query = self.search.trim().to_lowercase();
            let tracks: Vec<_> = queue
                .iter()
                .filter(|track| {
                    query.is_empty()
                        || format!("{} {} {}", track.title, track.artist, track.collection)
                            .to_lowercase()
                            .contains(&query)
                })
                .cloned()
                .collect();
            cache.queue = std::rc::Rc::new(queue);
            cache.tracks = std::rc::Rc::new(tracks);
            cache.key = Some(key);
            cache.revision += 1;
        }
        (cache.queue.clone(), cache.tracks.clone(), cache.revision)
    }
    pub fn engine_mut(&mut self) -> &mut MpvEngine {
        &mut self.engine
    }
    pub fn finish_services(&mut self) {
        if let Some(service) = self.online.service.take() {
            service.finish();
        }
        self.online_stop_proxy();
        for proxy in self.online.retired.drain(..) {
            proxy.finish();
        }
        self.cancel_hold();
        if self.settings_dirty.is_some() || self.history_dirty.is_some() {
            self.save_with_notice(self.settings_dirty.is_some());
        }
        if let Some(persistence) = self.persistence.take() {
            persistence.finish();
        }
        if let Some(service) = self.library_service.take() {
            service.finish();
        }
        if let Some(service) = self.video_service.take() {
            service.finish();
        }
        self.appearance_observer = None;
        self.font_task = None;
        if let Some(t) = self.thumbnails.take() {
            t.finish();
        }
        if let Some(backgrounds) = self.backgrounds.take() {
            backgrounds.finish();
        }
        self.dialogs = None;
        self.assets_service = None;
    }
}
#[derive(Default)]
struct QueueProjection {
    key: Option<(u64, i32, u64, Option<Duration>, String, bool)>,
    queue: std::rc::Rc<Vec<MediaPreview>>,
    tracks: std::rc::Rc<Vec<MediaPreview>>,
    revision: u64,
}

fn source_title(source: &MediaSource) -> String {
    match source {
        MediaSource::Local(path) => path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        MediaSource::Url(url) => url.clone(),
    }
}
fn format_time(duration: Option<Duration>) -> String {
    match duration {
        None => "--:--".into(),
        Some(duration) => {
            let seconds = duration.as_secs();
            if seconds >= 3600 {
                format!(
                    "{}:{:02}:{:02}",
                    seconds / 3600,
                    seconds / 60 % 60,
                    seconds % 60
                )
            } else {
                format!("{}:{:02}", seconds / 60, seconds % 60)
            }
        }
    }
}

#[cfg(test)]
mod navigation_tests {
    use super::*;

    fn controller() -> AppController {
        AppController::new(MpvEngine::default(), player_platform::current())
    }

    #[test]
    fn settings_returns_to_online_search_without_resetting_the_query_or_mode() {
        let mut c = controller();
        c.dispatch(UiAction::Navigate(6));
        c.online.query = "保留搜索条件".into();
        c.online.lyric_mode = true;
        c.online.busy = true;
        c.control("settings", "");
        assert_eq!(c.view_model().page, 3);
        assert!(!c.online.busy);
        c.control("settings", ""); // No duplicate visit for the current page.
        c.control("page-back", "");
        let view = c.view_model();
        assert_eq!(view.page, 6);
        assert_eq!(view.online.query, "保留搜索条件");
        assert!(view.online.lyric_mode);
        c.control("page-back", "");
        assert_eq!(c.view_model().page, 0);
        assert!(!c.view_model().can_go_back);
        c.control("page-back", "");
        assert_eq!(c.view_model().page, 0);
    }

    #[test]
    fn lyrics_and_settings_returns_follow_the_route_and_restore_expanded_state() {
        let mut c = controller();
        c.dispatch(UiAction::Navigate(6));
        c.control("music-detail", "");
        c.dispatch(UiAction::Navigate(3));
        c.dispatch(UiAction::Navigate(2));
        for page in [3, 4, 6, 0] {
            c.control("page-back", "");
            assert_eq!(c.view_model().page, page);
            assert_eq!(c.expanded, page == 4);
        }
        assert!(!c.view_model().can_go_back);
        c.dispatch(UiAction::Navigate(6));
        c.control("music-detail", "");
        c.native_window_mode(true, false);
        c.control("page-back", "");
        assert_eq!(c.view_model().page, 6);
        assert_eq!(c.window_mode, 0);
        assert_eq!(c.take_window_request(), Some(0));
    }

    #[test]
    fn back_shortcut_consumes_one_visit_and_does_not_repeat_or_replace_escape() {
        let mut c = controller();
        c.dispatch(UiAction::Navigate(6));
        c.dispatch(UiAction::Navigate(3));
        assert!(c.key("Alt+Left", true, true, true));
        assert_eq!(c.view_model().page, 3);
        assert!(c.key("Alt+Left", true, false, true));
        assert_eq!(c.view_model().page, 6);
        assert!(c.key("Alt+Left", true, true, true));
        assert_eq!(c.view_model().page, 6);
        c.panel_open = true;
        assert!(c.key("Escape", true, false, false));
        assert!(!c.panel_open);
        assert_eq!(c.view_model().page, 6);
        assert!(!c.key("Alt+Left", false, false, true));
        // An existing user binding keeps its meaning, including input blocking.
        c.settings.shortcuts.seek_back = "Alt+Left".into();
        assert!(!c.key("Alt+Left", true, false, true));
        assert_eq!(c.view_model().page, 6);
    }
}
