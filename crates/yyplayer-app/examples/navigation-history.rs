//! Actual Slint clicks wired to production AppController navigation.
//! Uses the layout preview controller without media or user configuration.
#[path = "../src/controller.rs"]
#[allow(dead_code)]
mod controller;
#[path = "../src/demo.rs"]
mod demo;
#[path = "../src/services.rs"]
#[allow(dead_code)]
mod services;

use player_ui::{UiAction, UiShell};
use slint::{ComponentHandle, Timer, TimerMode};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

fn click(w: &player_ui::AppWindow, x: f32, y: f32) {
    let position = slint::LogicalPosition::new(x, y);
    w.window()
        .dispatch_event(slint::platform::WindowEvent::PointerPressed {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
    w.window()
        .dispatch_event(slint::platform::WindowEvent::PointerReleased {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()?;
    std::fs::create_dir_all("target/navigation-history")?;
    let c = Rc::new(RefCell::new(controller::AppController::new(
        player_mpv::MpvEngine::default(),
        player_platform::current(),
    )));
    let ui = UiShell::new()?;
    let dispatch_c = c.clone();
    let project = ui.projector();
    let dispatch = Rc::new(move |a: UiAction| {
        dispatch_c.borrow_mut().dispatch(a);
        project(&dispatch_c.borrow().view_model());
    });
    ui.bind(dispatch.clone());
    ui.bind_controls(dispatch);
    ui.project(&c.borrow().view_model());
    let weak = ui.component().as_weak();
    let project = ui.projector();
    let failure = Rc::new(RefCell::new(None::<String>));
    let errors = failure.clone();
    let complete = Rc::new(Cell::new(false));
    let done = complete.clone();
    let timer = Timer::default();
    let mut stage = 0;
    let mut since = Instant::now();
    let mut records = Vec::new();
    let expected = [
        0, 6, 3, 6, 3, 2, 5, 2, 3, 6, 6, 3, 6, 4, 6, 4, 6, 0, 0, 6, 3, 6, 3, 6,
    ];
    timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let w = weak.upgrade().ok_or("Window closed")?;
            let m = c.borrow().view_model();
            project(&m);
            if since.elapsed() > Duration::from_secs(6) {
                return Err(format!("stage {stage}: expected page {}, actual {}, opacity {}", expected[stage], m.page, w.get_content_opacity()).into());
            }
            if since.elapsed() < Duration::from_millis(600) || w.get_content_opacity() < 0.99 {
                return Ok(());
            }
            if m.page != expected[stage] {
                return Err(format!("stage {stage}: expected page {}, actual {}", expected[stage], m.page).into());
            }
            let size = w.window().size().to_logical(w.window().scale_factor());
            if (w.get_content_width() - (size.width - w.get_content_left())).abs() > 1. {
                return Err("Page content lost its available width".into());
            }
            if m.can_go_back != !matches!(stage, 0 | 17 | 18) {
                return Err(format!("stage {stage}: wrong Back availability").into());
            }
            if m.page == 6 && stage > 1 && m.online.query != "保留搜索条件" {
                return Err("Returning reset the search query".into());
            }
            records.push(serde_json::json!({"stage":stage,"page":m.page,"can_go_back":m.can_go_back,"query":m.online.query,"opacity":w.get_content_opacity(),"width":w.get_content_width(),"window_width":size.width}));
            if matches!(stage, 2 | 3 | 9 | 21 | 22) {
                let p = w.window().take_snapshot()?;
                image::save_buffer(format!("target/navigation-history/stage-{stage}.png"),p.as_bytes(),p.width(),p.height(),image::ColorType::Rgba8)?;
            }
            match stage {
                0 | 18 => click(&w, 100., 266.),
                1 => {
                    w.invoke_control("online-query".into(), "保留搜索条件".into());
                    click(&w, 100., size.height - 152.);
                }
                3 | 10 | 19 | 21 => click(&w, 100., size.height - 152.),
                4 => click(&w, 100., 316.),
                5 => click(&w, 100., 216.),
                9 => click(&w, 100., 266.), // Repeated visit must not push.
                12 | 14 | 15 => click(&w, 44., size.height - 40.),
                // Without lyrics, NowPlaying centers its artwork across the page.
                13 => click(&w, size.width * 0.5, size.height * 0.4),
                17 => {
                    click(&w, 100., size.height - 197.); // Disabled at stack root.
                    w.window().set_size(slint::LogicalSize::new(1000., 640.));
                }
                20 => {
                    click(&w, 100., size.height - 197.);
                    w.window().set_size(slint::LogicalSize::new(1460., 920.));
                }
                23 => {
                    std::fs::write("target/navigation-history/results.json",serde_json::to_vec_pretty(&records)?)?;
                    done.set(true);
                    println!("PASS: production controller / actual Slint Back clicks / nested LIFO / duplicate visit / search preservation / artwork returns / stack root / three window sizes");
                    slint::quit_event_loop()?;
                }
                _ => click(&w, 100., size.height - 197.),
            }
            stage += 1;
            since = Instant::now();
            Ok(())
        })();
        if let Err(error) = result {
            *errors.borrow_mut() = Some(error.to_string());
            let _ = slint::quit_event_loop();
        }
    });
    ui.run()?;
    if let Some(error) = failure.borrow_mut().take() {
        return Err(error.into());
    }
    if !complete.get() {
        return Err("Navigation history regression interrupted".into());
    }
    Ok(())
}
