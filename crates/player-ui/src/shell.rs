use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::{AppWindow, TrackRow, view_model::ShellViewModel};

#[derive(Clone, Debug)]
pub enum UiAction {
    Navigate(i32),
    Search(String),
    SelectPreview(i32),
    OpenFile,
    RequestPlayback,
    Previous,
    Next,
    SetVolume(f32),
    ToggleFavorite,
    Control(String, String),
    ShortcutEdited(i32, String),
}

pub struct UiShell {
    window: AppWindow,
}

impl UiShell {
    pub fn new() -> Result<Self, slint::PlatformError> {
        Ok(Self {
            window: AppWindow::new()?,
        })
    }

    pub fn bind(&self, dispatch: Rc<dyn Fn(UiAction)>) {
        let handler = dispatch.clone();
        self.window
            .on_navigate(move |page| handler(UiAction::Navigate(page)));
        let handler = dispatch.clone();
        self.window
            .on_search_changed(move |text| handler(UiAction::Search(text.into())));
        let handler = dispatch.clone();
        self.window
            .on_select_preview(move |id| handler(UiAction::SelectPreview(id)));
        let handler = dispatch.clone();
        self.window
            .on_open_file(move || handler(UiAction::OpenFile));
        let handler = dispatch.clone();
        self.window
            .on_request_playback(move || handler(UiAction::RequestPlayback));
        let handler = dispatch.clone();
        self.window.on_previous(move || handler(UiAction::Previous));
        let handler = dispatch.clone();
        self.window.on_next(move || handler(UiAction::Next));
        let handler = dispatch.clone();
        self.window
            .on_volume_changed(move |volume| handler(UiAction::SetVolume(volume)));
        self.window
            .on_toggle_favorite(move || dispatch(UiAction::ToggleFavorite));
    }

    pub fn bind_controls(&self, dispatch: Rc<dyn Fn(UiAction)>) {
        let control = dispatch.clone();
        self.window.on_control(move |action, value| {
            control(UiAction::Control(action.into(), value.into()))
        });
        self.window.on_shortcut_edited(move |index, value| {
            dispatch(UiAction::ShortcutEdited(index, value.into()))
        });
    }

    pub fn project(&self, state: &ShellViewModel) {
        project(&self.window, state);
    }

    /// The callback holds a weak window; UI callbacks never form a strong cycle.
    pub fn projector(&self) -> impl Fn(&ShellViewModel) + 'static {
        let weak = self.window.as_weak();
        move |state| {
            if let Some(window) = weak.upgrade() {
                project(&window, state);
            }
        }
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        self.window.window().show()?;
        let result = slint::run_event_loop();
        // Drop the UI's borrowed video image before the backend suspends its GL
        // renderer. Wayland suspension holds a mutable backend-window borrow;
        // changing UI properties from RenderingTeardown would re-enter it.
        self.window.set_video_frame(slint::Image::default());
        self.window.set_render_ready(false);
        let hidden = self.window.window().hide();
        result.and(hidden)
    }

    /// Window access for future native presenters and the developer preview example.
    /// Regular application callbacks should go through `bind` and `project`.
    pub fn component(&self) -> &AppWindow {
        &self.window
    }
}

fn rows(
    current: ModelRc<TrackRow>,
    items: &[crate::view_model::MediaPreview],
) -> ModelRc<TrackRow> {
    let next = items
        .iter()
        .map(|item| TrackRow {
            item_id: item.id,
            title: item.title.as_str().into(),
            artist: item.artist.as_str().into(),
            collection: item.collection.as_str().into(),
            duration: item.duration.as_str().into(),
            cover: item.cover,
            artwork: item.artwork.clone(),
        })
        .collect::<Vec<_>>();
    if let Some(model) = current.as_any().downcast_ref::<VecModel<TrackRow>>()
        && model.row_count() == next.len()
    {
        for (index, row) in next.into_iter().enumerate() {
            if model.row_data(index).as_ref() != Some(&row) {
                model.set_row_data(index, row);
            }
        }
        current
    } else {
        Rc::new(VecModel::from(next)).into()
    }
}

fn project(window: &AppWindow, state: &ShellViewModel) {
    let online = &state.online;
    if window.get_online_revision() != online.revision as i32 {
        window.set_online_tracks(rows(window.get_online_tracks(), &online.rows));
        window.set_online_revision(online.revision as i32);
    }
    if !same_strings(&window.get_online_providers(), &online.providers) {
        window.set_online_providers(strings(&online.providers));
    }
    if !same_strings(&window.get_online_sources(), &online.sources) {
        window.set_online_sources(strings(&online.sources));
    }
    if !same_strings(&window.get_online_history(), &online.history) {
        window.set_online_history(strings(&online.history));
    }
    if !same_strings(&window.get_online_qualities(), &online.qualities) {
        window.set_online_qualities(strings(&online.qualities));
    }
    if !same_strings(&window.get_online_directories(), &online.directories) {
        window.set_online_directories(strings(&online.directories));
    }
    window.set_online_provider(online.provider);
    window.set_online_source(online.source);
    window.set_online_selected(online.selected);
    window.set_online_keep_history(online.keep_history);
    window.set_online_music_enabled(online.music_enabled);
    window.set_online_lyrics_enabled(online.lyrics_enabled);
    window.set_online_busy(online.busy);
    window.set_online_more(online.more);
    window.set_online_lyric_mode(online.lyric_mode);
    window.set_online_download(online.download);
    window.set_online_download_play(online.download_play);
    if window.get_online_query().as_str() != online.query {
        window.set_online_query(online.query.as_str().into());
    }
    if window.get_online_message().as_str() != online.message {
        window.set_online_message(online.message.as_str().into());
    }
    if window
        .get_online_quality_enabled()
        .iter()
        .collect::<Vec<_>>()
        != online.quality_enabled
    {
        window.set_online_quality_enabled(
            Rc::new(VecModel::from(online.quality_enabled.clone())).into(),
        );
    }
    window.set_window_controls_left(state.window_controls_left);
    let fonts = &state.fonts;
    if window.get_font_revision() != fonts.revision as i32 {
        let mut names = fonts.names.as_ref().clone();
        if names.is_empty() {
            names.push(String::new());
        }
        names[0] = "系统默认".into();
        if !same_strings(&window.get_font_names(), &names) {
            window.set_font_names(strings(&names));
            names[0] = "跟随全局字体".into();
            window.set_font_lyric_names(strings(&names));
            names[0] = "播放器默认（sans-serif）".into();
            window.set_font_subtitle_names(strings(&names));
        }
        window.set_ui_font(fonts.ui.as_str().into());
        window.set_lyric_font(fonts.lyrics.as_str().into());
        window.set_font_ui_index(fonts.ui_index);
        window.set_font_lyric_index(fonts.lyrics_index);
        window.set_font_subtitle_index(fonts.subtitle_index);
        window.set_font_ass(fonts.override_ass);
        window.set_font_busy(fonts.busy);
        window.set_font_message(fonts.message.as_str().into());
        window.set_font_revision(fonts.revision as i32);
    }
    let a = &state.appearance;
    window.set_appearance_design(a.design);
    window.set_appearance_scheme(a.scheme);
    window.set_appearance_dark(a.dark);
    window.set_appearance_accent(slint::Color::from_rgb_u8(
        (a.accent >> 16) as u8,
        (a.accent >> 8) as u8,
        a.accent as u8,
    ));
    window.set_appearance_ink(slint::Color::from_rgb_u8(
        (a.ink >> 16) as u8,
        (a.ink >> 8) as u8,
        a.ink as u8,
    ));
    window.set_appearance_accent_text(slint::Color::from_rgb_u8(
        (a.accent_text >> 16) as u8,
        (a.accent_text >> 8) as u8,
        a.accent_text as u8,
    ));
    window.set_appearance_system_accent(a.system_accent);
    window.set_appearance_custom(a.custom_accent.as_str().into());
    window.set_appearance_reduced(a.reduce_motion);
    window.set_appearance_os_reduced(a.os_reduce_motion);
    window.set_appearance_system_available(a.system_available);
    window.set_library_background_style(a.library_background_style);
    window.set_library_background_opacity(a.library_background_opacity);
    window.set_library_background_blur(a.library_background_blur);
    window.set_library_background_name(a.library_background_name.as_str().into());
    window.set_lyrics_background_style(a.lyrics_background_style);
    window.set_lyrics_background_opacity(a.lyrics_background_opacity);
    window.set_lyrics_background_blur(a.lyrics_background_blur);
    window.set_lyrics_background_name(a.lyrics_background_name.as_str().into());
    if window.get_background_image_revision() != a.background_image_revision as i32 {
        window.set_custom_library_background(a.custom_library_background.clone());
        window.set_custom_lyrics_background(a.custom_lyrics_background.clone());
        window.set_background_image_revision(a.background_image_revision as i32);
    }
    let lib = &state.library;
    if window.get_library_revision() != lib.revision as i32 {
        window.set_library_tracks(rows(window.get_library_tracks(), &lib.rows));
        window.set_library_revision(lib.revision as i32);
    }
    if !same_strings(&window.get_library_folders(), &lib.folders) {
        window.set_library_folders(strings(&lib.folders));
    }
    window.set_library_folder(lib.folder);
    window.set_library_selected(lib.selected);
    window.set_library_busy(lib.busy);
    window.set_video_renderer_requested(state.video_renderer_requested);
    window.set_can_resume_video(state.can_resume_video);
    window.set_search_query(state.search_query.as_str().into());
    let video = &state.video_library;
    if window.get_video_library_revision() != video.revision as i32 {
        window.set_video_library_tracks(rows(window.get_video_library_tracks(), &video.rows));
        window.set_video_library_revision(video.revision as i32);
    }
    if !same_strings(&window.get_video_library_folders(), &video.folders) {
        window.set_video_library_folders(strings(&video.folders));
    }
    window.set_video_library_folder(video.folder);
    window.set_video_library_selected(video.selected);
    window.set_video_library_busy(video.busy);
    let audio = &state.audio;
    let rgb = if audio.lyric_style.album_color {
        audio.cover_color.unwrap_or(state.appearance.accent)
    } else {
        state.appearance.accent
    };
    let color =
        |rgb: u32| slint::Color::from_rgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8);
    window.set_lyrics_accent(color(rgb));
    window.set_lyrics_accent_ink(color(player_core::appearance::ink(rgb)));
    window.set_lyrics_accent_text(color(player_core::appearance::text_accent(
        rgb,
        state.appearance.dark,
    )));
    if window.get_lyric_style_revision() != audio.lyric_style_revision as i32 {
        let s = &audio.lyric_style;
        window.set_lyric_style(crate::LyricStyle {
            album_color: s.album_color,
            bilingual: s.bilingual,
            line_gap: s.line_gap as i32,
            inactive_size: s.inactive_size as i32,
            inactive_opacity: s.inactive_opacity as i32,
            inactive_weight: s.inactive_weight as i32,
            active_size: s.active_size as i32,
            active_weight: s.active_weight as i32,
            secondary_weight: s.secondary_weight as i32,
            bilingual_gap: s.bilingual_gap as i32,
        });
        window.set_lyric_style_revision(audio.lyric_style_revision as i32);
    }
    window.set_preview(audio.preview);
    window.set_active_lyric(audio.active_lyric);
    window.set_audio_status(audio.status.as_str().into());
    window.set_eq_origin(audio.origin.as_str().into());
    window.set_eq_binding(audio.binding.as_str().into());
    window.set_eq_headroom_text(audio.headroom.as_str().into());
    if window.get_asset_revision() != audio.asset_revision as i32 {
        window.set_asset_revision(audio.asset_revision as i32);
        window.set_music_cover(audio.cover.clone());
        window.set_album(audio.album.as_str().into());
        window.set_plain_lyrics(audio.plain_lyrics.as_str().into());
        window.set_lyrics(
            Rc::new(VecModel::from(
                audio
                    .lyrics
                    .iter()
                    .map(|(text, milliseconds)| lyric_row(text, *milliseconds))
                    .collect::<Vec<_>>(),
            ))
            .into(),
        );
        window.set_plain_lyric_lines(
            Rc::new(VecModel::from(
                audio
                    .plain_lyrics
                    .lines()
                    .map(|text| lyric_row(text, -1))
                    .collect::<Vec<_>>(),
            ))
            .into(),
        );
    }
    window.set_page(state.page);
    window.set_can_go_back(state.can_go_back);
    window.set_selected_id(state.selected_id);
    window.set_selected_title(state.selected_title.as_str().into());
    window.set_selected_artist(state.selected_artist.as_str().into());
    window.set_selected_duration(state.selected_duration.as_str().into());
    window.set_selected_cover(state.selected_cover);
    window.set_volume(state.volume_percent);
    window.set_favorite(state.favorite);
    window.set_status_message(state.status.as_str().into());
    window.set_status_revision(state.status_revision as i32);
    if window.get_queue_revision() != state.queue_revision as i32 {
        window.set_tracks(rows(window.get_tracks(), &state.tracks));
        window.set_queue(rows(window.get_queue(), &state.queue));
        window.set_queue_revision(state.queue_revision as i32);
    }
    window.set_recent(rows(window.get_recent(), &state.recent));
    window.set_playing(state.playing);
    window.set_has_media(state.has_media);
    window.set_has_video(state.has_video);
    window.set_position_text(state.position_text.as_str().into());
    window.set_progress(state.progress);
    window.set_seekable(state.seekable);
    window.set_muted(state.muted);
    if !same_strings(&window.get_speed_choices(), &state.speed_choices) {
        window.set_speed_choices(strings(&state.speed_choices));
    }
    window.set_speed_index(state.speed_index);
    window.set_media_info(state.info.as_str().into());
    if !same_strings(&window.get_device_names(), &state.device_names) {
        window.set_device_names(strings(&state.device_names));
    }
    window.set_device_index(state.device_index);
    if !same_strings(&window.get_audio_tracks(), &state.audio_tracks) {
        window.set_audio_tracks(strings(&state.audio_tracks));
    }
    window.set_audio_index(state.audio_index);
    if !same_strings(&window.get_subtitle_tracks(), &state.subtitle_tracks) {
        window.set_subtitle_tracks(strings(&state.subtitle_tracks));
    }
    window.set_subtitle_index(state.subtitle_index);
    if !same_strings(&window.get_chapters(), &state.chapters) {
        window.set_chapters(strings(&state.chapters));
    }
    window.set_panel_open(state.panel_open);
    window.set_fullscreen_mode(state.fullscreen);
    window.set_window_mode(state.window_mode);
    if window.get_settings_revision() != state.settings_revision as i32 {
        window.set_column_song(if state.column_song > 0.0 {
            state.column_song
        } else {
            0.46
        });
        window.set_column_artist(if state.column_artist > 0.0 {
            state.column_artist
        } else {
            0.24
        });
        window.set_audio_form(crate::AudioForm {
            mode: audio.mode,
            preserve_rate: audio.preserve_rate,
            offset_ms: audio.offset_ms,
            scope: audio.eq_scope,
            name: audio.eq_name.as_str().into(),
            enabled: audio.eq_enabled,
            preamp: audio.preamp.as_str().into(),
            auto_headroom: audio.auto_headroom,
            bands: Rc::new(VecModel::from(
                audio
                    .bands
                    .iter()
                    .map(|(f, g, q, k)| crate::EqBandRow {
                        frequency: f.as_str().into(),
                        gain: *g,
                        q: q.as_str().into(),
                        kind: *k,
                    })
                    .collect::<Vec<_>>(),
            ))
            .into(),
            presets: strings(&audio.presets),
            preset_index: audio.preset_index,
        });
        window.set_settings_revision(state.settings_revision as i32);
        window.set_decode_mode(state.decode_mode);
        window.set_decode_threads(state.decode_threads);
        window.set_deinterlace(state.deinterlace);
        window.set_deband(state.deband);
        window.set_decode_scope(state.decode_scope);
        window.set_shortcut_rows(
            Rc::new(VecModel::from(
                state
                    .short_bindings
                    .iter()
                    .map(|(action, binding)| crate::ShortcutRow {
                        action: action.as_str().into(),
                        binding: binding.as_str().into(),
                    })
                    .collect::<Vec<_>>(),
            ))
            .into(),
        );
        window.set_seek_step(state.seek_step);
        window.set_volume_step(state.volume_step);
        window.set_hold_ms(state.hold_ms);
        window.set_hold_speed(state.hold_speed.as_str().into());
    }
    window.set_decode_origin(state.decode_origin.as_str().into());
}

fn lyric_row(text: &str, milliseconds: i32) -> crate::LyricRow {
    let (primary, secondary) = player_core::lyrics::bilingual_parts(text);
    crate::LyricRow {
        text: text.into(),
        primary: primary.into(),
        secondary: secondary.into(),
        milliseconds,
    }
}

fn strings(values: &[String]) -> ModelRc<slint::SharedString> {
    Rc::new(VecModel::from(
        values
            .iter()
            .map(|value| value.as_str().into())
            .collect::<Vec<_>>(),
    ))
    .into()
}
fn same_strings(model: &ModelRc<slint::SharedString>, values: &[String]) -> bool {
    model.row_count() == values.len()
        && values.iter().enumerate().all(|(index, value)| {
            model
                .row_data(index)
                .is_some_and(|row| row.as_str() == value)
        })
}
