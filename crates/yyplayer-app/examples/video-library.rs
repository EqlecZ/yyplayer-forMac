//! Real controller, Slint clicks and libmpv render leases; own fixtures / config only.
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
use slint::{ComponentHandle, Timer, TimerMode};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

fn control(c: &mut controller::AppController, a: &str, v: &str) {
    c.dispatch(UiAction::Control(a.into(), v.into()));
}
fn click(w: &player_ui::AppWindow, x: f32, y: f32) {
    for pressed in [true, false] {
        let position = slint::LogicalPosition::new(x, y);
        let button = slint::platform::PointerEventButton::Left;
        w.window().dispatch_event(if pressed {
            slint::platform::WindowEvent::PointerPressed { position, button }
        } else {
            slint::platform::WindowEvent::PointerReleased { position, button }
        });
    }
}
fn capture(w: &player_ui::AppWindow, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let p = w.window().take_snapshot()?;
    image::save_buffer(
        format!("target/video-library/{name}.png"),
        p.as_bytes(),
        p.width(),
        p.height(),
        image::ColorType::Rgba8,
    )?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = services::settings_path();
    let base = std::env::current_dir()?.join("target/video-library");
    if !path.starts_with(&base) {
        return Err("Use an independent target/video-library config".into());
    }
    let fixture = base.join("library").canonicalize()?;
    let loose = base.join("loose.mp4").canonicalize()?;
    let audio = base.join("silence.wav").canonicalize()?;
    let bad = base.join("bad.mp4");
    std::fs::write(&bad, b"deliberately invalid own video fixture")?;
    let bad = bad.canonicalize()?;
    services::atomic_save(&path, &Default::default())?;
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
    let state = c.clone();
    let project = ui.projector();
    let dispatch = Rc::new(move |a| {
        state.borrow_mut().dispatch(a);
        project(&state.borrow().view_model());
    });
    ui.bind(dispatch.clone());
    ui.bind_controls(dispatch);
    ui.project(&c.borrow().view_model());
    let weak = ui.component().as_weak();
    let project = ui.projector();
    let tick = c.clone();
    let timer = Timer::default();
    let fail = Rc::new(RefCell::new(None::<String>));
    let error = fail.clone();
    let complete = Rc::new(Cell::new(false));
    let success = complete.clone();
    let started = Instant::now();
    let mut due = started + Duration::from_millis(700);
    let mut stage = 0;
    let mut frames = 0;
    let mut position = 0.0;
    let mut cycles = 0;
    let mut returning = false;
    let mut pause_step = 0;
    let mut pause_position = 0.0;
    let mut playback_generation = 0;
    let mut renderer_initializations = 0;
    let mut records = Vec::new();
    timer.start(TimerMode::Repeated, Duration::from_millis(60), move || {
        let Some(w) = weak.upgrade() else { return; };
        let mut c = tick.borrow_mut(); c.render_failed(w.get_render_error().as_str()); c.tick(); project(&c.view_model());
        if Instant::now() < due { return; }
        let mut pending_click = None;
        let result = (|| -> Result<bool, Box<dyn std::error::Error>> {
            macro_rules! ensure { ($v:expr, $m:expr) => { let passed = $v; if !passed { return Err($m.into()); } }; }
            ensure!(started.elapsed() < Duration::from_secs(100), format!("timeout at stage {stage}: {}", c.view_model().status));
            let v = c.view_model(); let snapshot = c.engine_mut().snapshot().clone();
            let size = w.window().size().to_logical(w.window().scale_factor());
            ensure!(snapshot.error.is_empty() || stage == 17, snapshot.error.clone());
            ensure!(w.get_render_error().is_empty(), w.get_render_error().to_string());
            match stage {
                0 => {
                    if !snapshot.ready { return Ok(false); }
                    ensure!(!w.get_render_ready() && w.get_render_frames() == 0 && !v.video_renderer_requested, "startup initialized video renderer");
                    pending_click = Some((100., 208.));
                }
                1 => {
                    ensure!(v.page == 5 && !w.get_render_ready(), "sidebar must open video library without renderer");
                    if w.get_content_opacity() <= 0.99 { return Ok(false); }
                    ensure!(w.get_content_opacity() > 0.99 && w.get_content_width() > 800., "video library transparent / shrunk");
                    capture(&w, "empty")?;
                    c.prepare_video_paths(true, vec![fixture.clone(), fixture.join("A")]);
                    c.prepare_video_paths(false, vec![loose.clone()]);
                }
                2 => {
                    if v.video_library.busy || v.video_library.rows.len() != 3 { return Ok(false); }
                    ensure!(v.library.rows.is_empty() && w.get_render_frames() == 0, "video import polluted music or rendered video");
                    control(&mut c, "video-folder", "2");
                }
                3 => {
                    ensure!(v.video_library.rows.len() == 1, "folder filter failed");
                    c.dispatch(UiAction::Search("不存在的名字".into()));
                    ensure!(c.view_model().video_library.rows.is_empty(), "video search failed");
                    c.dispatch(UiAction::Search(String::new())); control(&mut c, "video-folder", "0");
                }
                4 => {
                    ensure!(v.video_library.rows.len() == 3, "all videos filter failed");
                    capture(&w, "populated")?;
                    pending_click = Some((w.get_content_left() + 110., 302.));
                }
                5 => {
                    if !v.playing || !w.get_render_ready() || w.get_render_frames() < 12 { return Ok(false); }
                    ensure!(v.page == 1 && v.has_video && snapshot.video_output_enabled, "row did not launch real video player");
                    c.engine_mut().submit(PlaybackCommand::Seek(Duration::from_secs(3)))?;
                }
                6 => {
                    if snapshot.position.is_none_or(|p| p.as_secs_f64() < 3.) { return Ok(false); }
                    position = snapshot.position.unwrap().as_secs_f64(); capture(&w, "playing")?;
                    pending_click = Some((size.width - 296.5, 62.));
                }
                7 => {
                    if w.get_render_ready() || snapshot.video_output_enabled { return Ok(false); }
                    ensure!(v.page == 5 && snapshot.phase == PlaybackPhase::Idle && v.can_resume_video, format!("return state: page={}, phase={:?}, source={:?}, position={:?}", v.page, snapshot.phase, snapshot.source, snapshot.position));
                    ensure!(v.can_resume_video, "return lost resumable session");
                    frames = w.get_render_frames(); capture(&w, "returned")?;
                }
                8 => {
                    ensure!(w.get_render_frames() == frames, "hidden video continues rendering");
                    pending_click = Some((w.get_content_left() + 110., 302.));
                }
                9 => {
                    if !v.playing || !w.get_render_ready() || w.get_render_frames() < frames + 12 { return Ok(false); }
                    ensure!(snapshot.position.unwrap().as_secs_f64() >= position - 0.3, "recreated player restarted at zero");
                    capture(&w, "recreated")?;
                }
                10 => {
                    if returning {
                        if w.get_render_ready() || snapshot.video_output_enabled { return Ok(false); }
                        ensure!(snapshot.phase == PlaybackPhase::Idle && v.can_resume_video, "cycle return did not preserve suspended session");
                        let from_settings = cycles % 2 == 1;
                        ensure!(v.page == if from_settings { 3 } else { 5 }, "cycle returned to the wrong page");
                        frames = w.get_render_frames(); control(&mut c, if from_settings { "page-back" } else { "video" }, ""); returning = false;
                        return Ok(false);
                    }
                    if !w.get_render_ready() || !v.playing || w.get_render_frames() < frames + 8 { return Ok(false); }
                    cycles += 1;
                    ensure!(v.page == 1 && snapshot.position.is_some_and(|p| p.as_secs_f64() >= position - 0.3), "history return lost the video page or position");
                    if cycles < 10 { c.dispatch(UiAction::Navigate(if cycles % 2 == 1 { 3 } else { 5 })); returning = true; return Ok(false); }
                }
                11 => {
                    c.dispatch(UiAction::Navigate(5));
                }
                12 => {
                    if w.get_render_ready() || snapshot.video_output_enabled { return Ok(false); }
                    frames = w.get_render_frames();
                    c.open_paths(vec![loose.clone()]); c.dispatch(UiAction::Navigate(5));
                    c.open_paths(vec![audio.clone()]);
                }
                13 => {
                    if !v.playing || v.has_video || snapshot.source != Some(player_core::MediaSource::Local(audio.clone())) { return Ok(false); }
                    ensure!(!w.get_render_ready() && !v.video_renderer_requested && w.get_render_frames() == frames, "audio / cancelled load initialized renderer");
                    c.dispatch(UiAction::Navigate(5));
                    let id = v.video_library.rows[0].id.to_string(); control(&mut c, "video-remove", &id);
                }
                14 => {
                    if v.video_library.busy || v.video_library.rows.len() != 2 { return Ok(false); }
                    ensure!(v.playing && !v.has_video, "removing video stopped unrelated audio");
                    ensure!(fixture.join("A/湖边.mp4").exists(), "removal deleted a source file");
                    control(&mut c, "video-folder", "2"); control(&mut c, "video-folder-remove", "");
                    w.window().set_size(slint::LogicalSize::new(1000.,640.)); control(&mut c, "panel", "");
                }
                15 => {
                    if v.video_library.busy { return Ok(false); }
                    ensure!(w.get_content_opacity() > 0.99 && w.get_content_width() > 350., "minimum library / options layout failed");
                    capture(&w, "minimum-options")?;
                    let (saved, warning) = services::load(&path);
                    if saved.video_library.roots.len()!=1 || saved.video_library.excluded.len()!=1 { return Ok(false); }
                    ensure!(warning.is_empty() && saved.library.roots.is_empty(), "video settings did not persist independently");
                    c.dispatch(UiAction::Navigate(5)); control(&mut c, "video-rescan", ""); control(&mut c, "video-cancel", "");
                    ensure!(!c.view_model().video_library.busy, "scan cancellation stuck");
                }
                16 => { c.open_paths(vec![bad.clone()]); }
                17 => {
                    if v.status.is_empty() || w.get_render_ready() || v.video_renderer_requested { return Ok(false); }
                    ensure!(!snapshot.video_output_enabled, "failed playback retained video output");
                    c.open_paths(vec![audio.clone()]);
                }
                18 => {
                    if !v.playing || v.has_video || !v.status.is_empty() { return Ok(false); }
                    ensure!(!w.get_render_ready() && !v.video_renderer_requested, "audio recovery created renderer");
                }
                19 => {
                    w.set_pointer_in_controls(true);
                    let before = pause_step;
                    match pause_step {
                        0 => c.open_paths(vec![loose.clone()]),
                        1 => {
                            if !v.playing || !w.get_render_ready() { return Ok(false); }
                            playback_generation = snapshot.generation;
                            renderer_initializations = w.get_render_initializations();
                            pending_click = Some((size.width / 2., size.height - 23.));
                        }
                        2 => {
                            if snapshot.phase != PlaybackPhase::Paused { return Ok(false); }
                            ensure!(!v.playing && !w.get_playing(), "pause button did not project paused state");
                            pause_position = snapshot.position.unwrap().as_secs_f64();
                        }
                        3 => {
                            ensure!(snapshot.phase == PlaybackPhase::Paused, "normal pause was not retained");
                            ensure!((snapshot.position.unwrap().as_secs_f64()-pause_position).abs() < 0.05, "normal pause position moved");
                            pending_click = Some((size.width / 2., size.height - 23.));
                        }
                        4 => {
                            if !v.playing { return Ok(false); }
                            ensure!(snapshot.position.unwrap().as_secs_f64() > pause_position + 0.3, "normal resume restarted or stayed paused");
                            c.engine_mut().submit(PlaybackCommand::Seek(snapshot.duration.unwrap()+Duration::from_secs(1)))?;
                        }
                        5 => {
                            if snapshot.phase != PlaybackPhase::Ended { return Ok(false); }
                            ensure!(!v.playing, "EOF still projects playing");
                            pending_click = Some((size.width / 2., size.height - 23.));
                        }
                        6 => {
                            if !v.playing && snapshot.position.is_none_or(|p|p.as_secs_f64()<0.3) { return Ok(false); }
                            ensure!(snapshot.phase == PlaybackPhase::Playing && w.get_playing(), format!("EOF replay left stale phase {:?} at {:?}",snapshot.phase,snapshot.position));
                            pending_click = Some((size.width / 2., size.height - 23.));
                        }
                        7 => {
                            if snapshot.phase != PlaybackPhase::Paused { return Ok(false); }
                            pause_position = snapshot.position.unwrap().as_secs_f64();
                            capture(&w, "replay-paused")?;
                        }
                        8 => {
                            ensure!(snapshot.phase == PlaybackPhase::Paused && !w.get_playing(), "replay pause lost its state");
                            ensure!((snapshot.position.unwrap().as_secs_f64()-pause_position).abs() < 0.05, "replay pause position moved");
                            pending_click = Some((size.width / 2., size.height - 23.));
                        }
                        9 => {
                            if !v.playing { return Ok(false); }
                            ensure!(snapshot.position.unwrap().as_secs_f64() > pause_position + 0.3, "replay resume restarted at zero");
                            ensure!(snapshot.generation == playback_generation && w.get_render_initializations() == renderer_initializations, "pause/replay reloaded media or renderer");
                        }
                        _ => unreachable!(),
                    }
                    records.push(serde_json::json!({"pause_step":before,"phase":format!("{:?}",snapshot.phase),"position":snapshot.position.map(|p|p.as_secs_f64()),"playing":w.get_playing(),"generation":snapshot.generation,"initializations":w.get_render_initializations()}));
                    pause_step += 1;
                    if pause_step < 10 { due=Instant::now()+Duration::from_millis(900); return Ok(false); }
                }
                20 => {
                    records.push(serde_json::json!({"cycles":cycles,"result":"PASS"}));
                    std::fs::write(base.join("results.json"), serde_json::to_vec_pretty(&records)?)?;
                    println!("PASS: video library, real row/navigation clicks, lazy audio, release/recreate, resume, 10 cycles, rapid cancellation, remove, persistence, minimum layout, pause/resume before and after EOF");
                    success.set(true);
                    slint::quit_event_loop()?;
                }
                _ => unreachable!(),
            }
            records.push(serde_json::json!({"stage":stage,"page":v.page,"frames":w.get_render_frames(),"initializations":w.get_render_initializations(),"renderer":w.get_render_ready(),"video_output":snapshot.video_output_enabled,"position":snapshot.position.map(|p|p.as_secs_f64()),"videos":v.video_library.rows.len()}));
            Ok(true)
        })();
        match result {
            Ok(true) => { stage += 1; due = Instant::now() + Duration::from_millis(800); }
            Ok(false) => {}
            Err(e) => { *error.borrow_mut()=Some(format!("stage {stage}: {e}")); let _=slint::quit_event_loop(); }
        }
        drop(c);
        if let Some((x,y))=pending_click { click(&w,x,y); }
    });
    ui.run()?;
    timer.stop();
    drop(timer);
    ui.component().window().hide()?;
    drop(ui);
    c.borrow_mut().engine_mut().finish()?;
    c.borrow_mut().finish_services();
    if let Some(e) = fail.borrow_mut().take() {
        return Err(e.into());
    }
    if !complete.get() {
        return Err("Video library test interrupted".into());
    }
    // A fresh controller restores the video index without any presenter attached.
    let mut restored =
        controller::AppController::live(player_mpv::MpvEngine::start(), player_platform::current());
    for _ in 0..150 {
        restored.tick();
        if !restored.view_model().video_library.rows.is_empty()
            && !restored.view_model().video_library.busy
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let v = restored.view_model();
    let ok = v.video_library.rows.len() == 2 && !v.video_renderer_requested;
    restored.engine_mut().finish()?;
    restored.finish_services();
    if !ok {
        return Err("fresh controller did not restore video library without renderer".into());
    }
    Ok(())
}
