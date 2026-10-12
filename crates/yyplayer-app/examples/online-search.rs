//! Actual Slint/controller/libmpv + self-owned HTTPS. Run via Test-Online.ps1.
#[path = "../src/controller.rs"]
#[allow(dead_code)]
mod controller;
#[path = "../src/demo.rs"]
mod demo;
#[path = "../src/services.rs"]
#[allow(dead_code)]
mod services;
use player_core::{PlaybackEngine, PlaybackPhase};
use player_ui::{UiAction, UiShell};
use slint::{ComponentHandle, Timer, TimerMode};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};
fn control(c: &mut controller::AppController, a: &str, v: &str) {
    c.dispatch(UiAction::Control(a.into(), v.into()));
}
fn click(w: &player_ui::AppWindow, x: f32, y: f32) {
    let position = slint::LogicalPosition::new(x, y);
    let button = slint::platform::PointerEventButton::Left;
    w.window()
        .dispatch_event(slint::platform::WindowEvent::PointerPressed { position, button });
    w.window()
        .dispatch_event(slint::platform::WindowEvent::PointerReleased { position, button });
}
fn key(w: &player_ui::AppWindow, text: &str) {
    w.window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.into() });
    w.window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: text.into() });
}
fn scroll_bottom(w: &player_ui::AppWindow) {
    w.window()
        .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
            position: slint::LogicalPosition::new(w.get_content_left() + 250., 400.),
            delta_x: 0.,
            delta_y: -100_000.,
        });
}
fn click_search(w: &player_ui::AppWindow) {
    let size = w.window().size().to_logical(w.window().scale_factor());
    click(w, size.width - 284., 88.);
}
fn shot(w: &player_ui::AppWindow, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let b = w.window().take_snapshot()?;
    image::save_buffer(
        format!("target/online-qa/{name}.png"),
        b.as_bytes(),
        b.width(),
        b.height(),
        image::ColorType::Rgba8,
    )?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let frontend_only = std::env::args().any(|a| a == "--frontend-only");
    let root = std::env::current_dir()?.join("target/online-qa");
    let path = PathBuf::from(
        std::env::var_os("YYPLAYER_CONFIG")
            .ok_or("YYPLAYER_CONFIG required; no user settings allowed")?,
    );
    if !path.starts_with(&root) {
        return Err("Independent QA config must be inside target/online-qa".into());
    }
    let mut s = player_core::settings::Settings::default();
    s.online.music_enabled = true;
    s.online.lyrics_enabled = true;
    s.online.api_directory = root.join("api");
    s.online.download_directory = root.join("ui-download");
    s.online.lyric_directory = root.join("ui-lyrics");
    s.online.artwork_directory = root.join("ui-artwork");
    s.online.stream_directory = root.join(format!("ui-stream-{}", std::process::id()));
    services::atomic_save(&path, &s)?;
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()?;
    let c = Rc::new(RefCell::new(controller::AppController::live(
        player_mpv::MpvEngine::start(),
        player_platform::current(),
    )));
    let ui = UiShell::new()?;
    ui.component()
        .window()
        .set_size(slint::LogicalSize::new(1240., 900.));
    let dispatch_c = c.clone();
    let project = ui.projector();
    let dispatch = Rc::new(move |a| {
        dispatch_c.borrow_mut().dispatch(a);
        project(&dispatch_c.borrow().view_model());
    });
    ui.bind(dispatch.clone());
    ui.bind_controls(dispatch);
    let weak = ui.component().as_weak();
    let state = c.clone();
    let failure = Rc::new(RefCell::new(None::<String>));
    let errors = failure.clone();
    let complete = Rc::new(Cell::new(false));
    let done = complete.clone();
    let tick_project = ui.projector();
    let timer = Timer::default();
    let mut stage = if frontend_only { 14 } else { 0 };
    let mut since = Instant::now();
    let mut captured = 0.;
    let mut records = vec![];
    let mut quality_ready = None::<Instant>;
    let mut stable_revision = 0;
    let mut stable_since = Instant::now();
    let mut folder_notice = 0;
    let mut saved_search = None;
    timer.start(TimerMode::Repeated,Duration::from_millis(100),move||{
        let Some(w)=weak.upgrade()else{return;};
        let result=(||->Result<(),Box<dyn std::error::Error>>{
            let m={let mut c=state.borrow_mut();c.tick();c.request_online_covers(w.get_online_visible_first(),w.get_online_visible_count());c.view_model()};
            tick_project(&m);
            // Winit/FemtoVG snapshots read the last back buffer. Refresh both
            // buffers over timer turns before capturing a paused/static page.
            w.window().request_redraw();
            if stable_revision!=m.online.revision {stable_revision=m.online.revision;stable_since=Instant::now();}
            if since.elapsed()>Duration::from_secs(75){shot(&w,"timeout")?;return Err(format!("stage {stage} timeout: {} / {}; rows={}, visible_first={}, more={}, busy={}",m.online.message,m.status,m.online.rows.len(),w.get_online_visible_first(),m.online.more,m.online.busy).into());}
            let mut c=state.borrow_mut();let snapshot=c.engine_mut().snapshot().clone();
            // A virtual list can refine its estimated extent after a large wheel
            // event. Keep scrolling over event-loop turns until the stable end,
            // rather than assuming one huge delta reaches the final geometry.
            if m.online.more && !m.online.busy && since.elapsed()>Duration::from_millis(350)
                && ((stage==17 && m.online.rows.len()==20) || (stage==18 && m.online.rows.len()==40) || stage==21 || stage==23) {
                drop(c);scroll_bottom(&w);return Ok(());
            }
            match stage {
                0 if !m.online.providers.is_empty()&&snapshot.ready=>{drop(c);w.invoke_navigate(6);},
                1 if m.page==6 && w.get_content_opacity()>0.99=>{shot(&w,"history")?;drop(c);click(&w,w.get_content_left()+250.,88.);key(&w,"原创 HTTPS 验收");key(&w,"\n");},
                2 if m.online.rows.len()==2&&!m.online.busy&&m.online.rows.iter().all(|s|s.artwork.size().width>0)&&stable_since.elapsed()>Duration::from_millis(500)=>{shot(&w,"results")?;drop(c);w.invoke_control("online-play".into(),"1".into());},
                3 if snapshot.phase==PlaybackPhase::Playing && snapshot.position.is_some_and(|p|p.as_secs_f64()>1.) && !m.audio.lyrics.is_empty() && m.audio.cover.size().width>0=>{
                    if m.audio.cover_color.is_none() { return Err("Online artwork has no worker-extracted palette".into()); }
                    if !matches!(snapshot.source,Some(player_core::MediaSource::Url(_))){return Err("First playback must be a real stream".into());}
                    records.push(serde_json::json!({"stage":"stream","position":snapshot.position.map(|p|p.as_secs_f64()),"video":snapshot.video,"title":m.selected_title,"cover_color":m.audio.cover_color.map(|c|format!("#{c:06x}"))}));
                    c.dispatch(UiAction::Navigate(4));
                },
                4 if m.page==4&&w.get_content_opacity()>0.99&&since.elapsed()>Duration::from_millis(650)=>{shot(&w,"lyrics")?;control(&mut c,"seek-percent","50");c.dispatch(UiAction::RequestPlayback);},
                5 if snapshot.phase==PlaybackPhase::Paused && snapshot.position.is_some_and(|p|p.as_secs_f64()>=5.5) && since.elapsed()>Duration::from_millis(600)=>{captured=snapshot.position.map(|p|p.as_secs_f64()).unwrap_or(0.);c.dispatch(UiAction::Navigate(6));control(&mut c,"online-menu","1");},
                6 if m.page==6&&m.online.selected==1&&since.elapsed()>Duration::from_millis(650)=>{let pos=snapshot.position.map(|p|p.as_secs_f64()).unwrap_or(0.);if (pos-captured).abs()>0.05{return Err("Pause did not freeze the stream".into());}shot(&w,"quality-menu")?;control(&mut c,"online-check-qualities","");},
                7 if !m.online.busy&&m.online.qualities.iter().all(|q|q.contains("地址可用"))=>{if quality_ready.is_none(){quality_ready=Some(Instant::now());return Ok(());}
                    if quality_ready.is_some_and(|t|t.elapsed()<Duration::from_millis(500)){return Ok(());}shot(&w,"quality-checked")?;control(&mut c,"online-options-download","yes");control(&mut c,"online-quality","4");},
                8 if snapshot.phase==PlaybackPhase::Playing && matches!(snapshot.source,Some(player_core::MediaSource::Local(_))) && snapshot.position.is_some_and(|p|p.as_secs_f64()>0.7) && !m.library.rows.is_empty() && m.online.message.starts_with("已下载")=>{
                    let Some(player_core::MediaSource::Local(path))=snapshot.source.clone()else{unreachable!()};
                    if !path.is_file()||!path.with_extension("lrc").is_file()||!path.with_extension("jpg").is_file(){return Ok(());}
                    if m.library.rows[0].title!="原创 HTTPS 验收"{return Err("Downloaded metadata was not retained".into());}
                    records.push(serde_json::json!({"stage":"download-play","file":path,"position":snapshot.position.map(|p|p.as_secs_f64()),"lyrics":m.audio.lyrics.len()}));shot(&w,"downloaded")?;
                    control(&mut c,"online-music-enabled","no");control(&mut c,"online-lyrics","");
                },
                9 if m.page==6&&m.online.lyric_mode&&!m.online.busy=>{if snapshot.phase!=PlaybackPhase::Playing{return Err("Disabling music stopped a local download".into());}control(&mut c,"online-search","");},
                10 if m.online.lyric_mode&&!m.online.busy&&m.online.rows.len()==2=>{if m.online.history.len()!=1{return Err("Lyric candidate search entered music history".into());}control(&mut c,"online-play","1");},
                11 if m.page==4&&!m.online.busy&&m.audio.lyrics.first().is_some_and(|l|l.0=="手动候选版本")&&since.elapsed()>Duration::from_millis(650)=>{shot(&w,"manual-lyrics")?;records.push(serde_json::json!({"stage":"manual-local-lyrics","music_enabled":m.online.music_enabled,"first_line":m.audio.lyrics[0].0}));c.dispatch(UiAction::Navigate(3));},
                12 if m.page==3&&w.get_content_opacity()>0.99&&since.elapsed()>Duration::from_millis(650)=>{if !matches!(snapshot.source,Some(player_core::MediaSource::Local(_))){return Err("Local playback identity lost after manual lyrics".into());}drop(c);click(&w,w.get_content_left()+417.,178.);},
                13 if m.page==3&&since.elapsed()>Duration::from_millis(650)=>{shot(&w,"settings")?; control(&mut c,"stop","");control(&mut c,"online-history-clear","");},
                14 if !m.has_media && m.online.history.is_empty() && !m.online.providers.is_empty() && snapshot.ready=>{control(&mut c,"online-music-enabled","yes");c.dispatch(UiAction::Navigate(6));control(&mut c,"online-query","自动分页验收");},
                15 if m.page==6&&w.get_content_opacity()>0.99&&since.elapsed()>Duration::from_millis(650)=>{drop(c);click_search(&w);},
                16 if m.online.rows.len()==20&&!m.online.busy&&since.elapsed()>Duration::from_millis(650)=>{
                    if !m.online.more||w.get_online_visible_first()!=0{return Err("First page fetched before reaching the bottom".into());}
                    shot(&w,"pagination-first")?;drop(c);scroll_bottom(&w);
                },
                17 if m.online.rows.len()==40&&!m.online.busy&&since.elapsed()>Duration::from_millis(650)=>{
                    if w.get_online_visible_first()<1{return Err("Appending the next page reset the scroll position".into());}
                    records.push(serde_json::json!({"stage":"pagination-append","rows":40,"visible_first":w.get_online_visible_first()}));
                    shot(&w,"pagination-second")?;drop(c);scroll_bottom(&w);
                },
                18 if m.online.rows.len()==45&&!m.online.busy&&since.elapsed()>Duration::from_millis(650)=>{
                    if m.online.more{return Err("Partial final page still offers more".into());}drop(c);scroll_bottom(&w);
                },
                19 if since.elapsed()>Duration::from_millis(800)=>{
                    if m.online.rows.len()!=45||m.online.busy||m.online.more{return Err("End of results triggered another page".into());}
                    records.push(serde_json::json!({"stage":"pagination-end","rows":45,"more":false}));w.window().set_size(slint::LogicalSize::new(1240.,2000.));control(&mut c,"online-search","");stage=189;since=Instant::now();return Ok(());
                },
                189 if m.online.rows.len()==40&&!m.online.busy&&since.elapsed()>Duration::from_millis(800)=>{
                    if w.get_online_visible_first()!=0{return Err("Short-page fill required a scroll".into());}
                    records.push(serde_json::json!({"stage":"short-page-fill","rows":40,"visible_first":0}));w.window().set_size(slint::LogicalSize::new(1240.,900.));control(&mut c,"online-query","重复分页验收");control(&mut c,"online-search","");stage=20;since=Instant::now();return Ok(());
                },
                20 if m.online.rows.len()==20&&!m.online.busy&&since.elapsed()>Duration::from_millis(650)=>{drop(c);scroll_bottom(&w);},
                21 if m.online.rows.len()==20&&!m.online.busy&&!m.online.more&&since.elapsed()>Duration::from_millis(800)=>{
                    records.push(serde_json::json!({"stage":"duplicate-page-stop","rows":20,"more":false}));control(&mut c,"online-query","失败分页验收");control(&mut c,"online-search","");
                },
                22 if m.online.rows.len()==20&&!m.online.busy&&since.elapsed()>Duration::from_millis(650)=>{drop(c);scroll_bottom(&w);},
                23 if !m.online.busy&&!m.online.more&&m.online.message.contains("503")&&since.elapsed()>Duration::from_millis(800)=>{
                    if m.online.rows.len()!=20{return Err("Failed page discarded existing results".into());}
                    records.push(serde_json::json!({"stage":"failed-page-stop","rows":20,"message":m.online.message}));control(&mut c,"online-history-clear","");
                },
                24 if m.online.history.is_empty()=>{
                    saved_search=Some((m.online.query.clone(),m.online.provider,m.online.source,m.online.rows.iter().map(|r|r.title.clone()).collect::<Vec<_>>()));
                    if m.online.rows.len()!=20{return Err("Settings Back fixture must have populated results".into());}
                    c.dispatch(UiAction::Navigate(3));stage=240;since=Instant::now();return Ok(());
                },
                240 if m.page==3&&w.get_content_opacity()>0.99&&since.elapsed()>Duration::from_millis(650)=>{
                    drop(c);click(&w,100.,703.);stage=241;since=Instant::now();return Ok(());
                },
                241 if m.page==6&&w.get_content_opacity()>0.99&&since.elapsed()>Duration::from_millis(650)=>{
                    let current=(m.online.query.clone(),m.online.provider,m.online.source,m.online.rows.iter().map(|r|r.title.clone()).collect::<Vec<_>>());
                    if saved_search.as_ref()!=Some(&current){return Err("Settings Back lost the search query/provider/source/results".into());}
                    shot(&w,"returned-search")?;records.push(serde_json::json!({"stage":"settings-back-search","page":m.page,"query":m.online.query,"provider":m.online.provider,"source":m.online.source,"rows":m.online.rows.len()}));
                    c.dispatch(UiAction::Navigate(3));stage=25;since=Instant::now();return Ok(());
                },
                25 if m.page==3&&w.get_content_opacity()>0.99&&since.elapsed()>Duration::from_millis(650)=>{drop(c);click(&w,w.get_content_left()+417.,178.);},
                26 if m.page==3&&since.elapsed()>Duration::from_millis(650)=>{shot(&w,"settings-open-folder")?;drop(c);click(&w,955.,454.);},
                27 if m.status=="已打开 API 配置文件夹"=>{folder_notice=m.status_revision;records.push(serde_json::json!({"stage":"open-api-directory","path":m.online.directories[0]}));control(&mut c,"online-directory-reset","0");},
                28 if !m.online.busy&&m.online.directories[0]!=root.join("api").to_string_lossy()&&since.elapsed()>Duration::from_millis(650)=>{drop(c);click(&w,955.,454.);},
                29 if m.status=="已打开 API 配置文件夹"&&m.status_revision>folder_notice&&since.elapsed()>Duration::from_millis(650)=>{
                    let expected=std::env::current_exe()?.parent().ok_or("Missing exe directory")?.join("config/cloud_api");
                    if !expected.is_dir()||m.online.directories[0]!=expected.to_string_lossy(){return Err("Opening the default API directory did not resolve/create the effective path".into());}
                    records.push(serde_json::json!({"stage":"open-default-api-directory","path":expected}));c.dispatch(UiAction::Navigate(6));w.window().set_size(slint::LogicalSize::new(1000.,640.));
                },
                30 if m.page==6&&w.get_content_opacity()>0.99&&since.elapsed()>Duration::from_millis(650)=>{shot(&w,"header-minimum")?;w.window().set_size(slint::LogicalSize::new(1460.,920.));},
                31 if since.elapsed()>Duration::from_millis(650)=>{shot(&w,"header-wide")?;control(&mut c,"panel","");w.window().set_size(slint::LogicalSize::new(1000.,640.));},
                32 if m.panel_open&&since.elapsed()>Duration::from_millis(650)=>{
                    shot(&w,"header-options-minimum")?;std::fs::write(root.join("ui-results.json"),serde_json::to_vec_pretty(&records)?)?;
                    if frontend_only {println!("PASS: frontend automatic pagination/scroll preservation/short/final/duplicate/error pages, custom/default API directory buttons and responsive layout (media regression skipped)");}
                    else {println!("PASS: real Slint/controller HTTPS search, header input/Enter/button/three sizes/options, automatic pagination/scroll preservation/short/final/duplicate/error pages, custom/default API directory buttons, artwork, lyrics, stream/pause, quality, download-play, offline playback and history");}
                    done.set(true);slint::quit_event_loop()?;
                },
                _=>return Ok(()),
            }
            stage+=1;since=Instant::now();Ok(())
        })();if let Err(e)=result{*errors.borrow_mut()=Some(e.to_string());let _=slint::quit_event_loop();}
    });
    ui.run()?;
    timer.stop();
    c.borrow_mut().engine_mut().finish()?;
    c.borrow_mut().finish_services();

    if complete.get() && !frontend_only {
        let loaded = services::load(&path).0;
        if loaded.audio.lyric_files.is_empty()
            || loaded.audio.lyric_files.values().any(|p| !p.is_file())
        {
            return Err("Manual lyric association was not persisted".into());
        }
    }
    if let Some(error) = failure.borrow_mut().take() {
        return Err(error.into());
    }
    if !complete.get() {
        return Err("Online regression interrupted".into());
    }
    Ok(())
}
