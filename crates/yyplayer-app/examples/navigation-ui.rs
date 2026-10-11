//! Actual Winit/FemtoVG clicks, empty-state actions and responsive navigation.
//! cargo run --locked --offline -p yyplayer-app --example navigation-ui
//! No media engine or user settings; generated screenshots are layout evidence only.
use player_ui::{UiAction, UiShell, view_model::ShellViewModel};
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
    std::fs::create_dir_all("target/navigation-ui")?;
    let ui = UiShell::new()?;
    let model = Rc::new(RefCell::new(ShellViewModel {
        page: 1,
        selected_title: "导航回归 · 空音乐库".into(),
        volume_percent: 70.,
        device_names: vec!["系统默认".into()],
        speed_choices: vec!["1x".into()],
        settings_revision: 1,
        seek_step: 5,
        volume_step: 5,
        hold_ms: 350,
        hold_speed: "3".into(),
        ..Default::default()
    }));
    let imported = Rc::new(Cell::new(0));
    let import = imported.clone();
    let projection = ui.projector();
    let state = model.clone();
    let dispatch = Rc::new(move |a: UiAction| {
        let mut m = state.borrow_mut();
        match a {
            UiAction::Navigate(p) => {
                m.page = p;
                m.panel_open = false;
            }
            UiAction::Control(a, _) if a == "panel" => m.panel_open = !m.panel_open,
            UiAction::Control(a, _) if a == "library-folder-add" => import.set(import.get() + 1),
            _ => {}
        }
        projection(&m);
    });
    ui.bind(dispatch.clone());
    ui.bind_controls(dispatch);
    ui.project(&model.borrow());
    let weak = ui.component().as_weak();
    let timer = Timer::default();
    let mut step = 0;
    let mut due = Instant::now() + Duration::from_millis(800);
    let mut deadline = due + Duration::from_secs(6);
    let failure = Rc::new(RefCell::new(None));
    let result = failure.clone();
    let complete = Rc::new(Cell::new(false));
    let success = complete.clone();
    let mut records = Vec::new();
    timer.start(TimerMode::Repeated,Duration::from_millis(50),move || {
        if Instant::now()<due {return;} let Some(w)=weak.upgrade() else{return;};
        let run=||->Result<(),Box<dyn std::error::Error>> {
            let size=w.window().size().to_logical(w.window().scale_factor());
            let page=w.get_page(); let opacity=w.get_content_opacity(); let left=w.get_content_left(); let width=w.get_content_width();
            // A synchronous GPU snapshot may delay the next Wayland frame and
            // queued Slint change/timer callbacks. Observe completion over event
            // loop iterations, while still failing a permanently invisible page.
            if opacity<0.99 {
                if Instant::now() >= deadline {return Err(format!("Page {page} is still transparent: {opacity}").into());}
                return Ok(());
            }
            let gap=if page!=1 && model.borrow().appearance.design==1 {12.} else {0.};
            let expected=size.width-left-if w.get_panel_open(){370.+gap}else{0.};
            if (width-expected).abs()>1. {return Err(format!("Page shrank: {width}, expected {expected}").into());}
            if page!=1 && model.borrow().appearance.design==1 && (size.height-104.-w.get_content_bottom()-10.).abs()>1. {return Err("Floating bottom margin missing".into());}
            let p=w.window().take_snapshot()?;
            image::save_buffer(format!("target/navigation-ui/stage-{step}.png"),p.as_bytes(),p.width(),p.height(),image::ColorType::Rgba8)?;
            records.push(serde_json::json!({"stage":step,"page":page,"opacity":opacity,"width":width,"window_width":size.width,"panel":w.get_panel_open()}));
            match step {
                0=>click(&w,size.width-296.5,62.), // video header: library
                1=>{if page!=5 {return Err("Video -> library click failed".into());} click(&w,100.,size.height-152.);},
                2=>{if page!=3 {return Err("Settings nav click failed".into());} click(&w,100.,166.);},
                3=>{if page!=0 {return Err("Music nav click failed".into());} click(&w,left+width/2.-67.,size.height/2.+83.);},
                4=>{if imported.get()!=1 {return Err("Empty-state add-directory click failed".into());} click(&w,size.width-35.,size.height-40.);},
                5=>{if !w.get_panel_open(){return Err("Player options click failed".into());} w.window().set_size(slint::LogicalSize::new(1000.,640.));},
                6=>w.window().set_size(slint::LogicalSize::new(1460.,920.)),
                7=>{let mut m=model.borrow_mut();m.panel_open=false;m.appearance.dark=false;m.appearance.reduce_motion=true;ui_projection(&w,&m);drop(m);click(&w,100.,size.height-152.);},
                8=>{if page!=3 {return Err("Reduced-motion settings click failed".into());} let mut m=model.borrow_mut();m.appearance.design=0;ui_projection(&w,&m);drop(m);click(&w,100.,156.);},
                9=>{if page!=0 {return Err("Simple-mode music click failed".into());}click(&w,100.,256.);},
                10=>{if page!=6 {return Err("Independent online search navigation click failed".into());} std::fs::write("target/navigation-ui/results.json",serde_json::to_vec_pretty(&records)?)?;success.set(true);println!("PASS: actual navigation / empty-directory button / right options / margins / three sizes / reduced motion / simple theme");slint::quit_event_loop()?;},
                _=>unreachable!(),
            }
            Ok(())
        };
        let mut run=run;
        if let Err(e)=run(){*result.borrow_mut()=Some(format!("stage {step}: {e}"));let _=slint::quit_event_loop();}
        // Only advance after recording and exercising this stage.
        if records.last().is_some_and(|r| r["stage"].as_i64()==Some(step)) {
            step+=1;due=Instant::now()+Duration::from_millis(800);deadline=due+Duration::from_secs(6);
        }
    });
    ui.run()?;
    if let Some(e) = failure.borrow_mut().take() {
        return Err(e.into());
    }
    if !complete.get() {
        return Err("Navigation regression interrupted".into());
    }
    Ok(())
}
fn ui_projection(w: &player_ui::AppWindow, m: &ShellViewModel) {
    w.set_panel_open(m.panel_open);
    w.set_appearance_dark(m.appearance.dark);
    w.set_appearance_reduced(m.appearance.reduce_motion);
    w.set_appearance_design(m.appearance.design);
}
