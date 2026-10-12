//! Real controller + local asset worker + Slint/FemtoVG, independent fixtures.
#[path = "../src/controller.rs"]
#[allow(dead_code)]
mod controller;
#[path = "../src/demo.rs"]
mod demo;
#[path = "../src/services.rs"]
#[allow(dead_code)]
mod services;

use player_core::{PlaybackCommand, PlaybackEngine, PlaybackPhase};
use player_ui::{UiAction, UiShell};
use slint::{ComponentHandle, Model, Timer, TimerMode};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

fn click(w: &player_ui::AppWindow, x: f32, y: f32) {
    for pressed in [true, false] {
        let position = slint::LogicalPosition::new(x, y);
        w.window().dispatch_event(if pressed {
            slint::platform::WindowEvent::PointerPressed {
                position,
                button: slint::platform::PointerEventButton::Left,
            }
        } else {
            slint::platform::WindowEvent::PointerReleased {
                position,
                button: slint::platform::PointerEventButton::Left,
            }
        });
    }
}
fn control(c: &Rc<RefCell<controller::AppController>>, a: &str, v: &str) {
    c.borrow_mut()
        .dispatch(UiAction::Control(a.into(), v.into()));
}
fn shot(w: &player_ui::AppWindow, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let p = w.window().take_snapshot()?;
    image::save_buffer(
        format!("target/lyrics-ui/{name}.png"),
        p.as_bytes(),
        p.width(),
        p.height(),
        image::ColorType::Rgba8,
    )?;
    Ok(())
}
fn motion_shot(
    w: &player_ui::AppWindow,
    name: &str,
) -> Result<std::thread::JoinHandle<image::ImageResult<()>>, Box<dyn std::error::Error>> {
    let p = w.window().take_snapshot()?;
    let (width, height) = (p.width(), p.height());
    let bytes = p.as_bytes().to_vec();
    let path = format!("target/lyrics-ui/{name}.png");
    Ok(std::thread::spawn(move || {
        image::save_buffer(path, &bytes, width, height, image::ColorType::Rgba8)
    }))
}
fn centered(w: &player_ui::AppWindow) -> Result<(), Box<dyn std::error::Error>> {
    if w.get_lyric_viewport_height() < 100.
        || (w.get_lyric_active_center() - w.get_lyric_viewport_height() / 2.).abs() > 2.
    {
        return Err(format!(
            "Active row drifted: center {}, viewport {}, scroll {}",
            w.get_lyric_active_center(),
            w.get_lyric_viewport_height(),
            w.get_lyric_scroll_y()
        )
        .into());
    }
    Ok(())
}
fn color_rgb(c: slint::Color) -> u32 {
    (u32::from(c.red()) << 16) | (u32::from(c.green()) << 8) | u32::from(c.blue())
}
fn near_color(a: u32, b: u32, tolerance: i32) -> bool {
    [0, 8, 16]
        .into_iter()
        .all(|shift| (((a >> shift) & 255) as i32 - ((b >> shift) & 255) as i32).abs() <= tolerance)
}
fn control_pixels(
    w: &player_ui::AppWindow,
    checkbox: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let image = w.window().take_snapshot()?;
    let size = w.window().size().to_logical(w.window().scale_factor());
    let scale = w.window().scale_factor();
    let pixel = |x: f32, y: f32| {
        let x = (x * scale).round() as usize;
        let y = (y * scale).round() as usize;
        let i = (y * image.width() as usize + x) * 4;
        (u32::from(image.as_bytes()[i]) << 16)
            | (u32::from(image.as_bytes()[i + 1]) << 8)
            | u32::from(image.as_bytes()[i + 2])
    };
    let expected = color_rgb(w.get_display_accent());
    let mut samples = vec![
        (
            "play_pause",
            pixel(size.width / 2. + 13., size.height - 40.),
        ),
        (
            "progress_thumb",
            pixel(
                86. + ((size.width - 172.) * w.get_progress() / 100.).clamp(5., size.width - 177.),
                size.height - 88.,
            ),
        ),
        ("volume_track", pixel(size.width - 164., size.height - 40.)),
    ];
    if checkbox {
        samples.push((
            "album_checkbox",
            pixel(w.get_lyric_check_x() - 5., w.get_lyric_check_y() - 5.),
        ));
    }
    for (name, actual) in &samples {
        if !near_color(expected, *actual, 16) {
            return Err(format!(
                "GPU {name}: #{actual:06x}, expected #{expected:06x}, global #{:06x}, all {samples:?}; panel {}x{}, check ({},{})",
                color_rgb(w.get_actual_theme_accent()),
                w.get_lyric_options_width(),
                w.get_lyric_options_height(),
                w.get_lyric_check_x(),
                w.get_lyric_check_y()
            )
            .into());
        }
    }
    Ok(
        serde_json::json!({"expected":format!("#{expected:06x}"),"samples":samples.into_iter().map(|(name, c)| (name, format!("#{c:06x}"))).collect::<std::collections::BTreeMap<_,_>>(),"ambient":format!("#{:06x}", pixel(size.width * 0.06, size.height * 0.25))}),
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let release_motion = std::env::args().any(|a| a == "--release-motion");
    let root = std::env::current_dir()?.join("target/lyrics-ui");
    let path = services::settings_path();
    if !path.starts_with(&root) {
        return Err("Set independent YYPLAYER_CONFIG in target/lyrics-ui".into());
    }
    let mut settings = player_core::settings::Settings::default();
    settings.appearance.scheme = player_core::appearance::Scheme::Dark;
    settings.appearance.reduce_motion = false;
    settings.appearance.system_accent = false;
    settings.appearance.accent = "#6875e8".into();
    settings.online.music_enabled = false;
    settings.online.lyrics_enabled = false;
    services::atomic_save(&path, &settings)?;
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()?;
    let c = Rc::new(RefCell::new(controller::AppController::live(
        player_mpv::MpvEngine::start(),
        player_platform::current(),
    )));
    let ui = UiShell::new()?;
    player_ui::presenter::attach(
        ui.component(),
        c.borrow_mut().engine_mut().render_endpoint(),
    )?;
    let events = Rc::new(RefCell::new(Vec::<(String, String)>::new()));
    let callback = c.clone();
    let callback_events = events.clone();
    let project = ui.projector();
    let dispatch = Rc::new(move |a: UiAction| {
        if let UiAction::Control(a, v) = &a {
            callback_events.borrow_mut().push((a.clone(), v.clone()));
        }
        callback.borrow_mut().dispatch(a);
        project(&callback.borrow().view_model());
    });
    ui.bind(dispatch.clone());
    ui.bind_controls(dispatch);
    c.borrow_mut()
        .open_paths(vec![root.join("media/coral.wav")]);
    let weak = ui.component().as_weak();
    let project = ui.projector();
    let timer = Timer::default();
    let failure = Rc::new(RefCell::new(None::<String>));
    let errors = failure.clone();
    let complete = Rc::new(Cell::new(false));
    let done = complete.clone();
    let test = c.clone();
    let mut stage = 0;
    let mut since = Instant::now();
    let mut records = Vec::new();
    let mut trajectory = Vec::new();
    let mut pixels = Vec::new();
    let mut transition_shots = std::collections::BTreeSet::new();
    let mut warmed_panels = std::collections::BTreeSet::new();
    let motion_writes = Rc::new(RefCell::new(Vec::new()));
    let pending_motion = motion_writes.clone();
    let mut coral = 0u32;
    let mut pixel_wait_error = None::<String>;
    timer.start(TimerMode::Repeated, Duration::from_millis(20), move || {
        let run = (|| -> Result<(), Box<dyn std::error::Error>> {
            let w = weak.upgrade().ok_or("Window closed")?;
            test.borrow_mut().tick();
            let m = test.borrow().view_model(); project(&m);
            // OpenGL take_snapshot reads the last back buffer. Keep requesting
            // frames in this qualification, including paused/static popovers,
            // so both buffers contain the current UI before pixel assertions.
            w.window().request_redraw();
            if since.elapsed() > Duration::from_secs(10) { return Err(format!("Stage timed out: {}", pixel_wait_error.as_deref().unwrap_or("UI or media state")).into()); }
            let elapsed = since.elapsed().as_millis();
            if matches!(stage, 2 | 12 | 13 | 14 | 15 | 18 | 19 | 20) {
                trajectory.push(serde_json::json!({"stage":stage,"ms":elapsed,"page":m.page,"progress":w.get_cover_progress(),"x":w.get_moving_cover_x(),"y":w.get_moving_cover_y(),"size":w.get_moving_cover_size(),"opacity":w.get_content_opacity(),"travelling":w.get_cover_travelling(),"theme_mix":w.get_lyrics_theme_mix(),"accent":format!("#{:06x}",color_rgb(w.get_display_accent())),"background":format!("#{:06x}",color_rgb(w.get_display_background()))}));
                if matches!(stage, 2 | 12) && w.get_lyrics_theme_mix() > 0.3 && w.get_lyrics_theme_mix() < 0.7 && transition_shots.insert(stage) {
                    pending_motion.borrow_mut().push(motion_shot(&w, if stage == 2 { "theme-enter-middle" } else { "theme-return-middle" })?);
                }
            }
            let wait = match stage { 13 => 120, 14 => 80, _ => 850 };
            if elapsed < wait { return Ok(()); }
            let size = w.window().size().to_logical(w.window().scale_factor());
            macro_rules! ensure { ($condition:expr, $message:expr) => { if !$condition { return Err($message.into()); } }; }
            macro_rules! pixels_ready { ($checkbox:expr) => {
                match control_pixels(&w, $checkbox) {
                    Ok(sample) => { pixels.push(sample); pixel_wait_error = None; },
                    Err(error) => { pixel_wait_error = Some(error.to_string()); return Ok(()); },
                }
            }; }
            // Conditional popup geometry can be initialized by its first GPU
            // snapshot. Give the backend a subsequent frame before pixels.
            if matches!(stage, 5 | 26 | 28 | 31 | 33) && warmed_panels.insert(stage) {
                let _ = w.window().take_snapshot()?;
                return Ok(());
            }
            match stage {
                0 => {
                    if !m.has_media || m.audio.cover_color.is_none() || m.audio.lyrics.len() != 20 { return Ok(()); }
                    coral = m.audio.cover_color.unwrap();
                    ensure!((coral >> 16) > ((coral >> 8) & 255), "Coral cover hue missing");
                    test.borrow_mut().engine_mut().submit(PlaybackCommand::Pause)?;
                    control(&test, "music-browse", "");
                }
                1 => { ensure!(m.page == 0 && !m.playing, "Browse / actual pause missing"); ensure!(near_color(color_rgb(w.get_display_accent()), m.appearance.accent, 2), "Browse theme was contaminated by artwork"); pixels_ready!(false); shot(&w, "browse")?; click(&w, 44., size.height - 40.); }
                2 => {
                    ensure!(m.page == 4 && w.get_content_opacity() > 0.99 && (w.get_cover_progress() - 1.).abs() < 0.001, "Entrance failed");
                    ensure!(trajectory.iter().any(|r| r["progress"].as_f64().is_some_and(|v| v > 0.05 && v < 0.95)), "Cover skipped its journey");
                    ensure!(trajectory.iter().any(|r| r["progress"].as_f64().is_some_and(|v| v > 1.001)), "Cover bounce missing");
                    ensure!(w.get_lyrics_theme_mix() > 0.999 && near_color(color_rgb(w.get_display_accent()), coral, 2), "Whole lyric page theme missing");
                    pixels_ready!(false);
                    centered(&w)?; shot(&w, "coral-bilingual")?;
                    click(&w, size.width * 0.75, 36. + 76. + w.get_lyric_viewport_height() / 2.);
                }
                3 => {
                    ensure!(events.borrow().iter().any(|(a, v)| a == "lyrics-seek" && v == "0"), "Click changed the source timestamp");
                    control(&test, "lyrics-seek", "9300");
                }
                4 => { ensure!(m.audio.active_lyric == 3, "Real seek / active line mismatch"); centered(&w)?; shot(&w, "long-bilingual")?; click(&w, size.width - 165., 73.); }
                5 => { ensure!(w.get_lyric_options_open() && w.get_shortcuts_blocked(), "Actual style button / focus guard failed"); pixels_ready!(true); shot(&w, "style-panel")?; click(&w, w.get_lyric_increase_x(), w.get_lyric_increase_y()); }
                6 => {
                    ensure!(m.audio.lyric_style.line_gap == 25, "Actual line-gap SpinBox edit failed");
                    for (a, v) in [("inactive-size", "24"), ("inactive-opacity", "35"), ("inactive-weight", "300"), ("active-size", "36"), ("active-weight", "800"), ("secondary-weight", "500"), ("bilingual-gap", "16")] { control(&test, &format!("lyrics-style-{a}"), v); }
                    click(&w, size.width * 0.5, 140.);
                }
                7 => {
                    ensure!(!w.get_lyric_options_open(), "Outside click did not close lyric styles");
                    centered(&w)?;
                    let (saved, error) = services::load(&path); ensure!(error.is_empty(), error);
                    ensure!(m.audio.lyric_style.secondary_weight == 500 && m.audio.lyric_style.bilingual_gap == 16, "Style projection failed");
                    if saved.lyrics != m.audio.lyric_style { pixel_wait_error = Some("Style persistence did not complete".into()); return Ok(()); }
                    pixel_wait_error = None;
                    shot(&w, "custom-typography")?;
                    w.window().set_size(slint::LogicalSize::new(1000., 640.)); control(&test, "panel", "");
                }
                8 => { ensure!(m.panel_open && size.width == 1000., "Minimum panel geometry missing"); centered(&w)?; shot(&w, "minimum-options")?; control(&test, "panel", ""); control(&test, "lyrics-style-active-size", "64"); control(&test, "lyrics-style-line-gap", "80"); control(&test, "lyrics-seek", "57300"); }
                9 => { ensure!(m.audio.active_lyric == 19, "Last line seek failed"); centered(&w)?; shot(&w, "large-last-line")?; control(&test, "lyrics-seek", "0"); }
                10 => { ensure!(m.audio.active_lyric == 0, "First line seek failed"); centered(&w)?; shot(&w, "large-first-line")?; control(&test, "lyrics-style-reset", ""); w.window().set_size(slint::LogicalSize::new(1240., 900.)); }
                11 => { centered(&w)?; click(&w, w.get_moving_cover_x() + w.get_moving_cover_size() / 2., w.get_moving_cover_y() + w.get_moving_cover_size() / 2.); }
                12 => { ensure!(m.page == 0, "Cover return navigation failed"); if w.get_cover_progress().abs() >= 0.001 || w.get_cover_travelling() { return Ok(()); } ensure!(!release_motion || trajectory.iter().filter(|v| v["stage"] == 12 && v["theme_mix"].as_f64().is_some_and(|t| t > 0.1 && t < 0.9)).count() >= 2, "Release return skipped theme intermediate frames"); ensure!(w.get_lyrics_theme_mix() == 0. && near_color(color_rgb(w.get_display_accent()), m.appearance.accent, 2), "Browse theme did not restore"); pixels_ready!(false); shot(&w, "returned-browse")?; click(&w, 44., size.height - 40.); }
                13 => { ensure!(m.page == 4 && w.get_cover_progress() > 0., "Rapid entry missing"); click(&w, w.get_moving_cover_x() + w.get_moving_cover_size() / 2., w.get_moving_cover_y() + w.get_moving_cover_size() / 2.); }
                14 => { ensure!(m.page == 0, "Rapid reversal failed"); click(&w, w.get_moving_cover_x() + w.get_moving_cover_size() / 2., w.get_moving_cover_y() + w.get_moving_cover_size() / 2.); }
                15 => { ensure!(m.page == 4, "Rapid re-entry navigation failed"); if w.get_content_opacity() < 0.99 || w.get_cover_travelling() { return Ok(()); } centered(&w)?; control(&test, "lyrics-style-bilingual", "no"); }
                16 => { ensure!(!m.audio.lyric_style.bilingual && m.audio.lyrics[0].0.contains(" / "), "Bilingual toggle altered source"); centered(&w)?; shot(&w, "inline-original")?; control(&test, "lyrics-style-bilingual", "yes"); control(&test, "appearance-scheme", "1"); }
                17 => { ensure!(!m.appearance.dark, "Light scheme missing"); centered(&w)?; shot(&w, "light-bilingual")?; test.borrow_mut().open_paths(vec![root.join("media/blue.wav")]); ensure!(test.borrow().view_model().audio.cover_color.is_none(), "Previous cover palette survived new media"); }
                18 => {
                    let Some(rgb) = m.audio.cover_color else { return Ok(()); };
                    ensure!(rgb != coral && (rgb & 255) > (rgb >> 16), "New album theme was not projected");
                    if !near_color(color_rgb(w.get_display_accent()), rgb, 2) { return Ok(()); }
                    if elapsed < 1450 { return Ok(()); }
                    pixels_ready!(false);
                    test.borrow_mut().engine_mut().submit(PlaybackCommand::Pause)?;
                    shot(&w, "blue-light")?; test.borrow_mut().open_paths(vec![root.join("media/no-cover.wav")]);
                }
                19 => {
                    if !m.has_media || !m.selected_title.contains("no-cover") || !m.audio.lyrics.is_empty() { return Ok(()); }
                    ensure!(m.audio.cover_color.is_none() && m.audio.cover.size().width == 0, "Missing cover didn't fall back");
                    ensure!(w.get_lyrics_accent().red() == ((m.appearance.accent >> 16) & 255) as u8, "Global theme fallback missing");
                    if !near_color(color_rgb(w.get_display_accent()), m.appearance.accent, 2) { return Ok(()); }
                    pixels_ready!(false);
                    test.borrow_mut().engine_mut().submit(PlaybackCommand::Pause)?; shot(&w, "no-cover")?; test.borrow_mut().open_paths(vec![root.join("media/plain.wav")]);
                }
                20 => {
                    if m.audio.plain_lyrics.is_empty() || m.audio.cover_color.is_none() { return Ok(()); }
                    ensure!(m.audio.lyrics.is_empty() && w.get_plain_lyric_lines().row_count() == 3, "Plain lyric display missing");
                    if !near_color(color_rgb(w.get_display_accent()), m.audio.cover_color.unwrap(), 2) { return Ok(()); }
                    pixels_ready!(false);
                    test.borrow_mut().engine_mut().submit(PlaybackCommand::Pause)?; shot(&w, "plain-bilingual")?;
                    control(&test, "appearance-motion", "yes"); control(&test, "music-toggle", "");
                }
                21 => { ensure!(m.page == 0 && m.appearance.reduce_motion && !w.get_cover_travelling() && w.get_cover_progress() == 0., "Reduced-motion return failed"); click(&w, 44., size.height - 40.); }
                22 => { ensure!(m.page == 4 && w.get_cover_progress() == 1. && w.get_content_opacity() == 1. && !w.get_cover_travelling(), "Reduced-motion entry failed"); ensure!(near_color(color_rgb(w.get_display_accent()), m.audio.cover_color.unwrap(), 2), "Reduced-motion palette failed"); shot(&w, "reduced-motion")?; test.borrow_mut().dispatch(UiAction::Navigate(3)); }
                23 => { ensure!(m.page == 3, "Settings page missing"); click(&w, w.get_content_left() + 526., 176.); }
                24 => { shot(&w, "settings-lyrics")?; ensure!(w.get_settings_section() == 4, "Actual settings lyrics tab click failed"); ensure!(near_color(color_rgb(w.get_display_accent()), m.appearance.accent, 2), "Settings inherited album theme"); control(&test, "music-toggle", ""); }
                25 => { ensure!(m.page == 4, "Theme test re-entry failed"); click(&w, size.width - 165., 73.); }
                26 => { ensure!(w.get_lyric_options_open(), "Theme style panel missing"); pixels_ready!(true); click(&w, w.get_lyric_check_x(), w.get_lyric_check_y()); }
                27 => { ensure!(!m.audio.lyric_style.album_color && near_color(color_rgb(w.get_display_accent()), m.appearance.accent, 2), "Actual album switch / whole-page fallback failed"); click(&w, w.get_lyric_check_x(), w.get_lyric_check_y()); }
                28 => { ensure!(m.audio.lyric_style.album_color && near_color(color_rgb(w.get_display_accent()), m.audio.cover_color.unwrap(), 2), "Actual album switch restoration failed"); pixels_ready!(true); click(&w, size.width * 0.5, 140.); control(&test, "appearance-scheme", "2"); control(&test, "appearance-motion", "no"); click(&w, size.width * 0.5, size.height - 40.); }
                29 => { ensure!(m.playing, "Actual themed play click failed"); pixels_ready!(false); shot(&w, "playing-themed")?; click(&w, size.width * 0.5, size.height - 40.); }
                30 => { ensure!(!m.playing, "Actual themed pause click failed"); click(&w, size.width - 282., size.height - 40.); }
                31 => { ensure!(w.get_shortcuts_blocked(), "Speed popup did not protect keyboard focus"); shot(&w, "speed-popup-themed")?; let index = m.speed_choices.iter().position(|s| s == "1.5x").ok_or("Speed choice missing")?; let popup_height = (m.speed_choices.len() as f32 * 32. + 8.).min(280.); click(&w, size.width - 304., size.height - 57. - popup_height + index as f32 * 32. + 16.); }
                32 => { ensure!(m.speed_choices[m.speed_index as usize] == "1.5x", "Actual themed speed popup selection failed"); control(&test, "speed", "1x"); control(&test, "panel", ""); }
                33 => { ensure!(m.panel_open, "Themed playback panel missing"); ensure!(m.speed_choices[m.speed_index as usize] == "1x" && w.get_actual_speed_label() == "1x", format!("Speed reset mismatch: actual {}, displayed {}", m.speed_choices[m.speed_index as usize], w.get_actual_speed_label())); shot(&w, "playback-panel-themed")?; control(&test, "panel", ""); }
                34 => { ensure!(w.get_render_initializations() == 0 && w.get_render_error().is_empty(), "Audio initialized an mpv renderer"); let snap = test.borrow_mut().engine_mut().snapshot().clone(); ensure!(snap.ready && snap.error.is_empty() && matches!(snap.phase, PlaybackPhase::Paused | PlaybackPhase::Playing), "Actual audio path failed"); ensure!(trajectory.iter().filter(|v| v["stage"] == 2 && v["theme_mix"].as_f64().is_some_and(|t| t > 0.1 && t < 0.9)).count() >= 2, "Page theme jumped directly between colors"); ensure!(trajectory.iter().filter(|v| v["stage"] == 18).filter_map(|v| v["accent"].as_str()).collect::<std::collections::BTreeSet<_>>().len() > 2, "Song theme did not interpolate"); control(&test, "background-lyrics-style", "2"); control(&test, "lyrics-style-line-gap", "31"); w.window().set_size(slint::LogicalSize::new(1000., 640.)); }
                35 => { ensure!(size.width == 1000. && w.get_lyrics_background_style() == 2, "Minimum legacy background missing"); pixels_ready!(false); shot(&w, "legacy-minimum-themed")?; click(&w, size.width - 165., 73.); }
                36 => { ensure!(w.get_lyric_options_open() && w.get_lyric_options_height() > 300., "Minimum style panel collapsed"); pixels_ready!(true); shot(&w, "styles-minimum-themed")?; w.window().dispatch_event(slint::platform::WindowEvent::PointerScrolled { position: slint::LogicalPosition::new(size.width - 150., 400.), delta_x: 0., delta_y: -800. }); }
                37 => { shot(&w, "styles-minimum-bottom")?; click(&w, size.width - 180., size.height - 155.); }
                38 => { ensure!(m.audio.lyric_style.line_gap == 24, "Minimum style scroll / real reset click failed"); w.window().dispatch_event(slint::platform::WindowEvent::PointerScrolled { position: slint::LogicalPosition::new(size.width - 150., 400.), delta_x: 0., delta_y: 800. }); }
                39 => { pixels_ready!(true); shot(&w, "styles-minimum-reset")?; click(&w, w.get_lyric_increase_x(), w.get_lyric_increase_y()); }
                40 => { ensure!(m.audio.lyric_style.line_gap == 25, "Style control did not follow reset value"); control(&test, "lyrics-style-reset", ""); control(&test, "background-lyrics-style", "0"); control(&test, "speed", "1.37x"); }
                41 => { ensure!(m.speed_choices[m.speed_index as usize] == "1.37x" && w.get_actual_speed_label() == "1.37x", "Non-preset actual speed display failed"); shot(&w, "speed-nonpreset-themed")?; control(&test, "speed", "1x"); }
                42 => { ensure!(m.speed_choices[m.speed_index as usize] == "1x" && w.get_actual_speed_label() == "1x", "Actual speed did not restore after non-preset"); done.set(true); slint::quit_event_loop()?; }
                _ => unreachable!(),
            }
            records.push(serde_json::json!({"stage":stage,"page":m.page,"active":m.audio.active_lyric,"cover_color":m.audio.cover_color.map(|c|format!("#{c:06x}")),"cover_progress":w.get_cover_progress(),"center":w.get_lyric_active_center(),"viewport":w.get_lyric_viewport_height(),"scroll":w.get_lyric_scroll_y(),"style":m.audio.lyric_style,"width":size.width,"height":size.height,"opacity":w.get_content_opacity(),"theme_mix":w.get_lyrics_theme_mix(),"accent":format!("#{:06x}",color_rgb(w.get_display_accent())),"surface":format!("#{:06x}",color_rgb(w.get_display_surface())),"actual_speed":m.speed_choices[m.speed_index as usize],"speed_label":w.get_actual_speed_label().to_string()}));
            std::fs::write(root.join("results.json"), serde_json::to_vec_pretty(&serde_json::json!({"stages":records,"trajectory":trajectory,"control_pixels":pixels,"events":*events.borrow()}))?)?;
            stage += 1; since = Instant::now(); Ok(())
        })();
        if let Err(error) = run {
            if let Some(w) = weak.upgrade() { let _ = shot(&w, "failure-frame"); }
            let _ = std::fs::write(root.join("failure.json"), serde_json::to_vec_pretty(&serde_json::json!({"stage":stage,"error":error.to_string(),"trajectory":trajectory,"events":*events.borrow()})).unwrap());
            *errors.borrow_mut() = Some(format!("stage {stage}: {error}")); let _ = slint::quit_event_loop();
        }
    });
    ui.run()?;
    timer.stop();
    drop(timer);
    drop(ui);
    c.borrow_mut().engine_mut().finish()?;
    c.borrow_mut().finish_services();
    for write in motion_writes.borrow_mut().drain(..) {
        write
            .join()
            .map_err(|_| "Motion evidence writer panicked")??;
    }
    if let Some(error) = failure.borrow_mut().take() {
        return Err(error.into());
    }
    if !complete.get() {
        return Err("Lyric qualification interrupted".into());
    }
    println!(
        "PASS: full album theme + GPU control pixels / Android ambient / page+song color interpolation / themed play,pause,speed,style clicks / style persistence / bilingual wrap+seek+centering / reversible cover bounce / sizes+schemes+missing cover+plain lyrics+reduced motion / normal exit"
    );
    Ok(())
}
