//! Actual Slint layout regression for active-only wrapping and bilingual gaps.
use slint::{ComponentHandle, Timer, TimerMode};
use std::{cell::RefCell, rc::Rc, time::Duration};

slint::slint! {
    import { LyricLine } from "../../player-ui/ui/components/lyric-line.slint";
    import { Theme } from "../../player-ui/ui/theme.slint";
    export component LayoutProbe inherits Window {
        width: 880px; height: 420px; default-font-size: 14px;
        background: #f2f4fa;
        in-out property <bool> selected;
        in-out property <length> line-width: 360px;
        out property <length> primary-height: long.primary-block-height;
        out property <length> gap: long.language-gap;
        out property <length> short-gap: short.language-gap;
        out property <length> row-height: long.height;
        init => { Theme.reduced-motion = true; Theme.dark = false; }
        Text { x: 40px; y: 20px; text: root.selected ? "当前行：独立换行与较宽双语间距" : "普通行：实际小字号与较窄双语间距"; color: #1d2234; }
        long := LyricLine {
            x: 40px; y: 70px; width: root.line-width;
            active: root.selected; accent: #267b80;
            line: {text: "You left me here for dead, dead / 你将我留在这里", primary: "You left me here for dead, dead", secondary: "你将我留在这里", milliseconds: 0};
            style: {album-color: false, bilingual: true, line-gap: 24, inactive-size: 22, inactive-opacity: 60, inactive-weight: 400, active-size: 32, active-weight: 700, secondary-weight: 400, bilingual-gap: 10};
        }
        short := LyricLine {
            x: 470px; y: 70px; width: 360px;
            active: root.selected; accent: #267b80;
            line: {text: "Follow the light / 沿着光", primary: "Follow the light", secondary: "沿着光", milliseconds: 0};
            style: long.style;
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()?;
    std::fs::create_dir_all("target/lyric-layout")?;
    let window = LayoutProbe::new()?;
    let weak = window.as_weak();
    let error = Rc::new(RefCell::new(None::<String>));
    let failure = error.clone();
    let timer = Timer::default();
    let mut stage = 0;
    let mut inactive_height = 0.;
    let mut records = Vec::new();
    timer.start(TimerMode::Repeated, Duration::from_millis(650), move || {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let w = weak.upgrade().ok_or("Layout window closed")?;
            w.window().request_redraw();
            let primary = w.get_primary_height();
            let gap = w.get_gap();
            if (gap - w.get_short_gap()).abs() > 0.01 {
                return Err("Wrapping changed the bilingual gap".into());
            }
            match stage {
                0 => {
                    inactive_height = primary;
                    if primary > 35. || (gap - 7.).abs() > 0.01 { return Err("Inactive line retained the active line's height / gap".into()); }
                }
                1 => {
                    if primary < inactive_height * 1.5 || (gap - 10.).abs() > 0.01 { return Err("Fixture must wrap only when selected, with a wider gap".into()); }
                }
                2 => {
                    if primary < inactive_height * 1.5 || (gap - 7.).abs() > 0.01 { return Err("Narrow inactive wrapping added phantom translation space".into()); }
                }
                _ => unreachable!(),
            }
            records.push(serde_json::json!({"stage":stage,"selected":w.get_selected(),"primary_height":primary,"row_height":w.get_row_height(),"language_gap":gap,"short_gap":w.get_short_gap()}));
            let pixels = w.window().take_snapshot()?;
            image::save_buffer(format!("target/lyric-layout/stage-{stage}.png"), pixels.as_bytes(), pixels.width(), pixels.height(), image::ColorType::Rgba8)?;
            if stage == 2 {
                std::fs::write("target/lyric-layout/results.json", serde_json::to_vec_pretty(&records)?)?;
                slint::quit_event_loop()?;
            } else if stage == 0 { w.set_selected(true); } else { w.set_selected(false); w.set_line_width(240.); }
            stage += 1;
            Ok(())
        })();
        if let Err(e) = result { *failure.borrow_mut() = Some(e.to_string()); let _ = slint::quit_event_loop(); }
    });
    window.run()?;
    timer.stop();
    if let Some(e) = error.borrow_mut().take() {
        return Err(e.into());
    }
    println!(
        "PASS: actual inactive / active-only / narrow wrapping; bilingual gaps independent of block height"
    );
    Ok(())
}
