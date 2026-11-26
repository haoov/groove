use groove_controllers::AppState;
use groove_gfx::Fonts;

use crate::input::Key;
use crate::layout::Layout;
use crate::tests::{ALT_SHIFT, full_app, press, window};
use crate::{Ui, view};

fn at(tick: u64) -> groove_ui_kit::base::ctx::Metrics {
    let mut metrics = window();
    metrics.tick = tick;
    metrics
}

fn configured(still: bool) -> AppState {
    let mut app = crate::tests::agent::sourced(full_app(), true, true);
    let mut config = app.config.config.clone().expect("a config");
    config.ui.reduce_motion = still;
    app.config.config = Some(config);
    app
}

#[test]
fn a_fold_stands_at_once() {
    let app = configured(false);
    let mut ui = Ui::default();
    ui.animate(&app, 0);
    ui.session.stowed = true;
    ui.animate(&app, 1000);
    assert!(!ui.motion.moving(1001), "no frames asked for");
    assert_eq!(
        Layout::of(at(1001), &ui).strip.h,
        Layout::of(at(1001), &ui).header.h
    );
}

#[test]
fn with_reduced_motion_the_palette_stands_at_once() {
    let app = configured(true);
    let mut ui = Ui::default();
    ui.animate(&app, 0);
    press(Key::Char(';'), ALT_SHIFT, &mut ui, &app);
    ui.animate(&app, 1000);
    assert!(!ui.motion.moving(1001));
}

#[test]
fn the_palette_fades_in_as_it_opens_then_stands_whole() {
    let app = configured(false);
    let mut ui = Ui::default();
    ui.animate(&app, 0);
    press(Key::Char(';'), ALT_SHIFT, &mut ui, &app);
    ui.animate(&app, 1000);
    let faded = |tick: u64| {
        let (frame, _) = view(&app, &ui, at(tick), &mut Fonts::embedded());
        let glass = frame.layers().iter().filter(|one| !one.glass.is_empty());
        glass.map(|one| one.fade).fold(0.0, f32::max)
    };
    let early = faded(1030);
    assert!(early > 0.0 && early < 1.0, "part way in: {early}");
    assert_eq!(faded(1140), 0.0, "whole once it has appeared");
}
