//! Layout/click qualification only. The displayed playback state is an explicit fixture.
use player_ui::{
    UiAction, UiShell,
    view_model::{AudioViewModel, ShellViewModel},
};
use slint::{ComponentHandle, Timer, TimerMode};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
fn click(window: &player_ui::AppWindow, x: f32, y: f32) {
    let position = slint::LogicalPosition::new(x, y);
    for pressed in [true, false] {
        window.window().dispatch_event(if pressed {
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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()?;
    std::fs::create_dir_all("target/audio-ui")?;
    let cover =
        slint::Image::load_from_path(std::path::Path::new("target/audio-validation/cover.png"))?;
    let lyrics = Rc::new(vec![
        ("A quiet place for your sound".into(), 0),
        ("Follow the lights along the lake".into(), 3000),
        ("Let the evening breathe".into(), 6000),
        ("Listen to the space between".into(), 9000),
        ("Your music, close to you".into(), 12000),
        ("The night unfolds".into(), 18000),
    ]);
    let state = Rc::new(RefCell::new(ShellViewModel {
        page: 0,
        selected_title: "Lake Lights".into(),
        selected_artist: "YYPlayer Sessions".into(),
        selected_duration: "3:24".into(),
        position_text: "1:08".into(),
        progress: 33.0,
        volume_percent: 70.0,
        playing: true,
        has_media: true,
        seekable: true,
        speed_index: 0,
        speed_choices: vec!["1x".into()],
        settings_revision: 1,
        audio: AudioViewModel {
            cover,
            asset_revision: 1,
            lyrics,
            active_lyric: 2,
            album: "布局验收 · 播放状态为示例".into(),
            status: "输出状态样例".into(),
            eq_name: "平直 · 不处理".into(),
            preamp: "0".into(),
            auto_headroom: true,
            bands: [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]
                .into_iter()
                .map(|f| (f.to_string(), 0.0, "1.414".into(), 0))
                .collect(),
            ..Default::default()
        },
        device_names: vec!["输出设备样例".into()],
        ..Default::default()
    }));
    let ui = Rc::new(UiShell::new()?);
    ui.project(&state.borrow());
    let events = Rc::new(RefCell::new(Vec::new()));
    let callback_state = state.clone();
    let callback_events = events.clone();
    let project = ui.projector();
    ui.bind_controls(Rc::new(move |event| {
        if let UiAction::Control(action, value) = event {
            callback_events.borrow_mut().push((action.clone(), value));
            let mut state = callback_state.borrow_mut();
            match action.as_str() {
                "music-toggle" => state.page = if state.page == 4 { 0 } else { 4 },
                "music-detail" => state.page = 4,
                "music-browse" | "page-back" => state.page = 0,
                "panel" => state.panel_open = !state.panel_open,
                _ => {}
            }
            state.can_go_back = state.page == 4;
            project(&state);
        }
    }));
    let weak = ui.component().as_weak();
    let started = Instant::now();
    let mut step = 0;
    let failed = Rc::new(RefCell::new(None::<String>));
    let failure = failed.clone();
    let timer = Timer::default();
    let timer_project = ui.projector();
    timer.start(TimerMode::Repeated, Duration::from_millis(200), move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let elapsed = started.elapsed().as_secs_f64();
        if elapsed < (step + 1) as f64 {
            return;
        }
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let size = window
                .window()
                .size()
                .to_logical(window.window().scale_factor());
            match step {
                0 => {
                    click(&window, 44.0, size.height - 40.0);
                    if state.borrow().page != 4 {
                        return Err("Cover did not expand music page".into());
                    }
                }
                1 => {
                    capture(&window, "lyrics")?;
                }
                2 => {
                    let mut s = state.borrow_mut();
                    s.audio.lyrics = Default::default();
                    s.audio.asset_revision += 1;
                    timer_project(&s);
                }
                3 => {
                    capture(&window, "without-lyrics")?;
                    window
                        .window()
                        .set_size(slint::LogicalSize::new(1000.0, 640.0));
                }
                4 => {
                    capture(&window, "minimum")?;
                    let mut s = state.borrow_mut();
                    s.audio.lyrics = Rc::new(vec![("Let the evening breathe".into(), 0)]);
                    s.audio.active_lyric = 0;
                    s.audio.asset_revision += 1;
                    s.panel_open = true;
                    timer_project(&s);
                }
                5 => {
                    capture(&window, "panel-minimum")?;
                    window
                        .window()
                        .set_size(slint::LogicalSize::new(1240.0, 900.0));
                }
                6 => {
                    capture(&window, "audio-options")?;
                    let mut s = state.borrow_mut();
                    s.panel_open = false;
                    timer_project(&s);
                }
                7 => {
                    // Lyrics are centered in the viewport below the 76px inset and
                    // 40px heading, including the native/custom window chrome.
                    let lyric_y = (size.height + window.get_chrome_height()
                        - window.get_controls_height())
                        / 2.0
                        + 41.0;
                    click(&window, size.width * 0.65, lyric_y);
                    click(&window, 44.0, size.height - 40.0);
                    if state.borrow().page != 0 {
                        return Err("Collapse did not return to browse".into());
                    }
                    if !events
                        .borrow()
                        .iter()
                        .any(|(action, _)| action == "lyrics-seek")
                    {
                        return Err("Lyric click did not request seek".into());
                    }
                    println!(
                        "PASS: cover expand, collapse, lyric click; five music layout captures"
                    );
                    slint::quit_event_loop()?;
                }
                _ => {}
            }
            Ok(())
        })();
        if let Err(e) = result {
            *failure.borrow_mut() = Some(e.to_string());
            let _ = slint::quit_event_loop();
        }
        step += 1;
    });
    ui.run()?;
    timer.stop();
    if let Some(error) = failed.borrow_mut().take() {
        return Err(error.into());
    }
    Ok(())
}
fn capture(window: &player_ui::AppWindow, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pixels = window.window().take_snapshot()?;
    image::save_buffer(
        format!("target/audio-ui/{name}.png"),
        pixels.as_bytes(),
        pixels.width(),
        pixels.height(),
        image::ColorType::Rgba8,
    )?;
    Ok(())
}
