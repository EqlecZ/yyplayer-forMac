#[path = "window_surface.rs"]
mod window_surface;
use crate::controller::AppController;
use player_core::{PlaybackCommand, PlaybackEngine};
use player_mpv::MpvEngine;
use player_ui::{UiAction, UiShell};
use slint::winit_030::{EventResult, WinitWindowAccessor, winit};
use slint::{ComponentHandle, Timer, TimerMode};
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()?;
    slint::set_xdg_app_id("yyplayer")?;
    let controller = Rc::new(RefCell::new(AppController::live(
        MpvEngine::start(),
        player_platform::current(),
    )));
    let ui = UiShell::new()?;
    let decoded_icon = image::load_from_memory(include_bytes!(
        "../../../assets/icons/yyplayer/liquid-orbit-disc/yyplayer-256.png"
    ))?
    .into_rgba8();
    let icon_width = decoded_icon.width();
    let icon_height = decoded_icon.height();
    let icon = winit::window::Icon::from_rgba(decoded_icon.into_raw(), icon_width, icon_height)?;
    player_ui::presenter::attach(
        ui.component(),
        controller.borrow_mut().engine_mut().render_endpoint(),
    )?;
    ui.project(&controller.borrow().view_model());
    let weak = ui.component().as_weak();
    let dispatch_controller = controller.clone();
    let project = ui.projector();
    let dispatch_weak = weak.clone();
    let dispatch = Rc::new(move |action| {
        let request = {
            let mut controller = dispatch_controller.borrow_mut();
            controller.dispatch(action);
            controller.take_window_request()
        };
        if let Some(window) = dispatch_weak.upgrade() {
            apply_window_mode(&window, request);
        }
        project(&dispatch_controller.borrow().view_model());
    });
    ui.bind(dispatch.clone());
    ui.bind_controls(dispatch);
    let chrome_window = weak.clone();
    let chrome_controller = controller.clone();
    player_ui::window_controls::bind(
        ui.component(),
        Rc::new(move |mode| {
            chrome_controller
                .borrow_mut()
                .dispatch(UiAction::Control("window-mode".into(), mode.to_string()));
            let request = chrome_controller.borrow_mut().take_window_request();
            if let Some(window) = chrome_window.upgrade() {
                apply_window_mode(&window, request);
            }
        }),
    );

    let dropped = Rc::new(RefCell::new(Vec::new()));
    let event_dropped = dropped.clone();
    let keyboard_controller = controller.clone();
    let keyboard_weak = weak.clone();
    let mut modifiers = winit::keyboard::ModifiersState::empty();
    let mut mouse_held = false;
    let mut touches = HashSet::new();
    ui.component()
        .window()
        .on_winit_window_event(move |_, event| {
            use winit::event::{ElementState, MouseButton, TouchPhase, WindowEvent};
            match event {
                WindowEvent::ModifiersChanged(changed) => modifiers = changed.state(),
                WindowEvent::Focused(false) => {
                    keyboard_controller.borrow_mut().cancel_hold();
                    mouse_held = false;
                    touches.clear();
                    if let Some(window) = keyboard_weak.upgrade() {
                        window.set_pointer_held(false);
                        window.set_pointer_in_controls(false);
                        window.set_pointer_in_header(false);
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if let Some(window) = keyboard_weak.upgrade() {
                        let native = window.window();
                        let bottom = (f64::from(native.size().height) - position.y)
                            / f64::from(native.scale_factor());
                        window.set_pointer_in_controls(
                            bottom >= 0.0 && bottom <= f64::from(window.get_controls_height()),
                        );
                        let top = position.y / f64::from(native.scale_factor());
                        window.set_pointer_in_header(
                            top >= 0.0
                                && top
                                    <= f64::from(window.get_header_height())
                                        + if window.get_fullscreen_mode() {
                                            0.0
                                        } else {
                                            36.0
                                        }
                                && position.x >= 0.0
                                && position.x / f64::from(native.scale_factor())
                                    < f64::from(window.get_video_width()),
                        );
                        window.invoke_pointer_activity();
                    }
                }
                WindowEvent::CursorLeft { .. } => {
                    if let Some(window) = keyboard_weak.upgrade() {
                        window.set_pointer_in_controls(false);
                        window.set_pointer_in_header(false);
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if *button == MouseButton::Left {
                        mouse_held = *state == ElementState::Pressed;
                    }
                    if let Some(window) = keyboard_weak.upgrade() {
                        window.set_pointer_held(mouse_held || !touches.is_empty());
                        window.invoke_pointer_activity();
                    }
                }
                WindowEvent::Touch(touch) => {
                    match touch.phase {
                        TouchPhase::Started => {
                            touches.insert(touch.id);
                        }
                        TouchPhase::Ended | TouchPhase::Cancelled => {
                            touches.remove(&touch.id);
                        }
                        TouchPhase::Moved => {}
                    }
                    if let Some(window) = keyboard_weak.upgrade() {
                        window.set_pointer_held(mouse_held || !touches.is_empty());
                        window.invoke_pointer_activity();
                    }
                }
                WindowEvent::MouseWheel { .. } | WindowEvent::Focused(true) => {
                    if let Some(window) = keyboard_weak.upgrade() {
                        window.invoke_pointer_activity();
                    }
                }
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    if let Some(window) = keyboard_weak.upgrade() {
                        window.set_pointer_in_controls(false);
                        window.set_pointer_in_header(false);
                    }
                }
                WindowEvent::DroppedFile(path) => event_dropped.borrow_mut().push(path.clone()),
                WindowEvent::KeyboardInput {
                    event,
                    is_synthetic: false,
                    ..
                } => {
                    if let Some(window) = keyboard_weak.upgrade()
                        && let Some(chord) = key_chord(&event.logical_key, modifiers)
                    {
                        if event.state == ElementState::Pressed {
                            window.invoke_pointer_activity();
                            if chord == "Escape" && window.get_lyric_options_open() {
                                if !event.repeat {
                                    window.invoke_dismiss_lyric_options();
                                    keyboard_controller.borrow_mut().cancel_hold();
                                }
                                return EventResult::PreventDefault;
                            }
                        }
                        let consumed = keyboard_controller.borrow_mut().key(
                            &chord,
                            event.state == ElementState::Pressed,
                            event.repeat,
                            window.get_shortcuts_blocked(),
                        );
                        if consumed {
                            return EventResult::PreventDefault;
                        }
                    }
                }
                _ => {}
            }
            EventResult::Propagate
        });

    let paths = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if !paths.is_empty() {
        controller.borrow_mut().prepare_paths(paths);
    }
    let timer = Timer::default();
    let tick_controller = controller.clone();
    let tick_weak = weak.clone();
    let project = ui.projector();
    let started = Instant::now();
    // Optional deterministic developer verification; defaults never alter normal playback.
    let quit_after = std::env::var("YYPLAYER_SMOKE_SECONDS")
        .ok()
        .and_then(|seconds| seconds.parse::<f64>().ok())
        .filter(|seconds| seconds.is_finite() && *seconds > 0.0);
    let smoke_script = std::env::var_os("YYPLAYER_SMOKE_SCRIPT").is_some();
    let audio_script = std::env::var_os("YYPLAYER_AUDIO_SCRIPT").is_some();
    let diagnostics = std::env::var_os("YYPLAYER_DIAGNOSTICS").map(PathBuf::from);
    let mut actions = 0u8;
    let mut checkpoints = Vec::new();
    let mut surface = window_surface::Surface::default();
    let mut window_icon_applied = false;
    timer.start(TimerMode::Repeated,Duration::from_millis(60),move || {
        let Some(window) = tick_weak.upgrade() else { return; };
        if !window_icon_applied
            && window
                .window()
                .with_winit_window(|native| native.set_window_icon(Some(icon.clone())))
                .is_some()
        {
            window_icon_applied = true;
        }
        surface.update(&window);
        let mut controller = tick_controller.borrow_mut();
        controller.render_failed(window.get_render_error().as_str());
        if window.get_shortcuts_blocked() { controller.cancel_hold(); }
        let paths = std::mem::take(&mut *dropped.borrow_mut()); if !paths.is_empty() { controller.prepare_paths(paths); }
        controller.request_library_covers(window.get_library_visible_first(),window.get_library_visible_count());
        controller.request_online_covers(window.get_online_visible_first(), window.get_online_visible_count());
        controller.tick();
        window.set_window_maximized(window.window().is_maximized());
        if smoke_script {
            let seconds = started.elapsed().as_secs_f64();
            if actions == 0 && seconds > 1.5 { controller.dispatch(UiAction::Control("speed".into(),"1.5x".into())); actions = 1; }
            if actions == 1 && seconds > 2.5 { controller.dispatch(UiAction::SetVolume(35.0)); actions = 2; }
            if actions == 2 && seconds > 3.5 { controller.dispatch(UiAction::Control("mute".into(),"".into())); controller.dispatch(UiAction::RequestPlayback); actions = 3; }
            if actions == 3 && seconds > 4.5 { controller.dispatch(UiAction::RequestPlayback); controller.dispatch(UiAction::Control("speed".into(),"1x".into())); actions = 4; }
            if actions == 4 && seconds > 5.5 { controller.dispatch(UiAction::Control("forward".into(),"".into())); actions = 5; }
            if actions == 5 && seconds > 6.5 { controller.dispatch(UiAction::Control("window-mode".into(),"1".into())); actions = 6; }
            if actions == 6 && seconds > 7.5 { controller.dispatch(UiAction::Control("fullscreen".into(),"".into())); actions = 7; }
            if actions == 7 && seconds > 8.5 { controller.dispatch(UiAction::Control("escape".into(),"".into())); actions = 8; }
            if actions == 8 && seconds > 9.5 { controller.dispatch(UiAction::RequestPlayback); actions = 9; }
            if actions == 9 && seconds > 10.5 {
                controller.dispatch(UiAction::Control("decode-scope".into(),"2".into()));
                controller.dispatch(UiAction::Control("decode-mode".into(),"1".into()));
                controller.dispatch(UiAction::Control("save-decode".into(),"".into()));
                actions = 10;
            }
            if actions == 10 && seconds > 12.5 { controller.dispatch(UiAction::RequestPlayback); actions = 11; }
        }
        if audio_script {
            let seconds = started.elapsed().as_secs_f64();
            let steps: &[(f64, &[(&str, &str)])] = &[
                (1.5, &[("music-detail", "")]),
                (2.5, &[("eq-name", "Validation"), ("eq-enabled", "yes"), ("eq-band", "5:g:4.5"), ("eq-apply", "")]),
                (4.5, &[("eq-store", ""), ("device", "1"), ("eq-scope", "1"), ("eq-name", "Device EQ"), ("eq-enabled", "yes"), ("eq-band", "3:g:-3"), ("eq-apply", "")]),
                (6.5, &[("eq-scope", "2"), ("eq-name", "File EQ"), ("eq-enabled", "yes"), ("eq-band", "5:g:-6"), ("eq-apply", "")]),
                (8.5, &[("eq-clear", ""), ("output-mode", "2")]),
                (10.5, &[("music-browse", "")]),
                (11.5, &[("output-mode", "0"), ("eq-scope", "1"), ("eq-clear", ""), ("eq-scope", "0"), ("eq-flat", ""), ("eq-apply", "")]),
            ];
            if let Some((time, controls)) = steps.get(actions as usize) && seconds > *time {
                for (action, value) in *controls { controller.dispatch(UiAction::Control((*action).into(), (*value).into())); }
                actions += 1;
            }
        }
        let request = controller.take_window_request();
        apply_window_mode(&window,request);
        window.window().with_winit_window(|native| controller.native_window_mode(native.fullscreen().is_some(),native.is_maximized()));
        project(&controller.view_model());
        if checkpoints.len() < 60 && checkpoints.len() <= started.elapsed().as_secs() as usize {
            let snapshot = controller.engine_mut().snapshot();
            checkpoints.push(serde_json::json!({ "seconds": started.elapsed().as_secs_f64(), "phase": format!("{:?}",snapshot.phase), "speed": snapshot.speed, "position": snapshot.position.map(|value| value.as_secs_f64()), "volume": snapshot.volume, "muted": snapshot.muted, "hwdec": snapshot.hwdec, "fullscreen": window.window().is_fullscreen(), "maximized": window.window().is_maximized(), "frames": window.get_render_frames(), "error": snapshot.error, "audio_status": snapshot.audio_status, "exclusive": snapshot.exclusive_confirmed, "eq_filter": snapshot.eq_filter }));
        }
        if !window.get_render_ready() && window.get_render_error().is_empty() { window.window().request_redraw(); }
        if quit_after.is_some_and(|seconds| started.elapsed().as_secs_f64() >= seconds) {
            if let Some(path) = diagnostics.as_ref() {
                let view = controller.view_model();
                let snapshot = controller.engine_mut().snapshot();
                let mut data = serde_json::json!({ "phase": format!("{:?}",snapshot.phase), "position": snapshot.position.map(|position| position.as_secs_f64()), "duration": snapshot.duration.map(|duration| duration.as_secs_f64()), "speed": snapshot.speed, "volume": snapshot.volume, "muted": snapshot.muted, "runtime": snapshot.runtime, "hwdec": snapshot.hwdec, "video": snapshot.video, "devices": snapshot.devices.iter().map(|device| &device.name).collect::<Vec<_>>(), "tracks": snapshot.tracks.len(), "render_frames": window.get_render_frames(), "render_error": window.get_render_error().as_str(), "error": snapshot.error, "media_info": snapshot.info, "file_tag_info":view.info, "device_ids":snapshot.devices.iter().map(|d|&d.id).collect::<Vec<_>>(), "audio_info": snapshot.audio_info, "audio_log": snapshot.audio_log, "audio_source_rate":snapshot.audio_source_rate, "audio_output_rate":snapshot.audio_output_rate, "audio_fallback":snapshot.audio_fallback, "audio_status": snapshot.audio_status, "exclusive": snapshot.exclusive_confirmed, "eq_filter": snapshot.eq_filter, "music_title": view.selected_title, "artist": view.selected_artist, "album": view.audio.album, "fonts": {"ui": view.fonts.ui, "lyrics": view.fonts.lyrics, "families": view.fonts.names.len().saturating_sub(1), "subtitle_actual": snapshot.subtitle_font, "ass_actual": snapshot.subtitle_font_overrides}, "library_columns": {"song": view.column_song, "artist": view.column_artist}, "window_surface": {"preference":surface.state.preference,"result":surface.state.result}, "decorated": window.window().with_winit_window(|w| w.is_decorated()), "lyric_count": view.audio.lyrics.len(), "active_lyric": view.audio.active_lyric, "page": view.page, "cover_width": view.audio.cover.size().width, "script_steps": actions, "fullscreen": window.window().is_fullscreen(), "maximized": window.window().is_maximized(), "checkpoints": checkpoints });
                data["render_lifecycle"] = serde_json::json!({"initializations":window.get_render_initializations(),"ready":window.get_render_ready(),"video_output_enabled":snapshot.video_output_enabled});
                data["ready"] = serde_json::json!(snapshot.ready);
                data["audio_blocked"] = serde_json::json!(snapshot.audio_blocked);
                data["status"] = serde_json::json!(view.status);
                data["window_system"] = serde_json::json!(window.window().with_winit_window(|native| {
                    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                    match native.window_handle().map(|h| h.as_raw()) {
                        Ok(RawWindowHandle::Wayland(_)) => "wayland",
                        Ok(RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_)) => "x11",
                        Ok(RawWindowHandle::Win32(_)) => "win32",
                        Ok(RawWindowHandle::AppKit(_)) => "appkit",
                        _ => "unknown",
                    }
                }));
                let _ = std::fs::write(path,serde_json::to_vec_pretty(&data).unwrap());
            }
            let _ = slint::quit_event_loop();
        }
    });
    let result = ui.run();
    timer.stop();
    drop(timer);
    controller.borrow_mut().cancel_hold();
    let _ = controller
        .borrow_mut()
        .engine_mut()
        .submit(PlaybackCommand::Shutdown);
    ui.component().window().hide()?;
    drop(ui); // RenderingTeardown releases render lease before worker destroys core.
    controller.borrow_mut().finish_services();
    let shutdown = controller.borrow_mut().engine_mut().finish();
    result?;
    shutdown?;
    Ok(())
}
fn apply_window_mode(window: &player_ui::AppWindow, mode: Option<i32>) {
    if let Some(mode) = mode {
        window.window().set_fullscreen(mode == 2);
        if mode != 2 {
            window.window().set_maximized(mode == 1);
        }
    }
}
fn key_chord(
    key: &winit::keyboard::Key,
    modifiers: winit::keyboard::ModifiersState,
) -> Option<String> {
    use winit::keyboard::{Key, NamedKey};
    let key = match key {
        Key::Named(named) => match named {
            NamedKey::Space => "Space".into(),
            NamedKey::ArrowLeft => "Left".into(),
            NamedKey::ArrowRight => "Right".into(),
            NamedKey::ArrowUp => "Up".into(),
            NamedKey::ArrowDown => "Down".into(),
            NamedKey::Enter => "Enter".into(),
            NamedKey::Escape => "Escape".into(),
            NamedKey::Tab => "Tab".into(),
            NamedKey::Home => "Home".into(),
            NamedKey::End => "End".into(),
            NamedKey::PageUp => "PageUp".into(),
            NamedKey::PageDown => "PageDown".into(),
            NamedKey::Backspace => "Backspace".into(),
            NamedKey::Delete => "Delete".into(),
            NamedKey::Insert => "Insert".into(),
            key => {
                let key = format!("{key:?}");
                if key.starts_with('F')
                    && key[1..]
                        .parse::<u8>()
                        .is_ok_and(|number| (1..=24).contains(&number))
                {
                    key
                } else {
                    return None;
                }
            }
        },
        Key::Character(text) if text.as_str() == " " => "Space".into(),
        Key::Character(text) => match text.as_str() {
            "+" => "Plus".into(),
            "-" => "Minus".into(),
            _ => text.to_uppercase(),
        },
        _ => return None,
    };
    let mut chord = String::new();
    if modifiers.control_key() {
        chord.push_str("Ctrl+");
    }
    if modifiers.alt_key() {
        chord.push_str("Alt+");
    }
    if modifiers.shift_key() {
        chord.push_str("Shift+");
    }
    chord.push_str(&key);
    Some(chord)
}
