//! The Kubernetes tab in the modes layout: the list or the object open in the middle under one
//! band, the pinned lists, the open objects and the kinds in the sidebar, the keys and the wheel.

use groove_controllers::{AppState, Command, session};
use groove_gfx::Fonts;
use groove_types::{LogSource, Pin};

use crate::hit::{Hits, Scroller, Target};
use crate::input::{Delta, Input, Key, Modifiers};
use crate::tests::resources::fixture::{crashing, lands, link};
use crate::tests::resources::{key, kind, listing, pods, row as object_row};
use crate::tests::{ALT, all_texts, click, handle, press, window};
use crate::views::session::Mode;
use crate::views::session::resources::{View, pins};

/// The listing of `staging` on `shop` and `cnpg`, with modes on `KUBERNETES`.
fn kubed() -> (AppState, crate::Ui) {
    let (mut app, mut ui) = listing();
    let config = app.config.config.as_mut().expect("a config");
    config.ui.modes = true;
    ui.session.mode = Mode::Kubernetes;
    ui.settle(&app);
    (app, ui)
}

fn texts(app: &AppState, ui: &crate::Ui) -> (Vec<String>, Hits) {
    let (frame, hits) = crate::view(app, ui, window(), &mut Fonts::embedded());
    (all_texts(&frame), hits)
}

fn hit(app: &AppState, ui: &mut crate::Ui, target: Target) -> Vec<Command> {
    let (_, hits) = texts(app, ui);
    let at = hits.rect_of(&target);
    let at = at.unwrap_or_else(|| panic!("{target:?} drawn"));
    click(at, ui, app, &hits)
}

fn row(uid: &str) -> Target {
    Target::ResourceRow(uid.into())
}

/// The list shown as a pin, and the session holding it.
fn pinned(app: &mut AppState, ui: &crate::Ui) -> Pin {
    let open = app.session.selected().expect("a session");
    let pin = pins::shown(app, open, &ui.session.resources).expect("a list shown");
    let id = app.session.selected.clone().expect("a session");
    let open = app.session.get_mut(&id).expect("open");
    open.pins.push(pin.clone());
    pin
}

#[test]
fn the_list_band_names_the_kind_its_count_and_its_scope_and_no_strip_of_tabs_stands() {
    let (app, ui) = kubed();
    let (said, hits) = texts(&app, &ui);
    for one in ["PODS · 3", "staging", "shop", "cnpg", "api-0", "NAME"] {
        assert!(said.iter().any(|text| text == one), "{one}: {said:?}");
    }
    assert!(hits.rect_of(&Target::ResourcePin).is_some(), "the pin");
    assert!(hits.rect_of(&Target::ResourceList).is_none(), "no strip");
    let band = hits.rect_of(&Target::ResourcePin).expect("the band");
    let first = hits.rect_of(&row("shop/api-0")).expect("a row");
    assert!(band.bottom() <= first.y, "the band over the rows");
}

#[test]
fn a_click_on_a_row_opens_the_object_under_open_and_the_middle_shows_it_with_its_views() {
    let (app, mut ui) = kubed();
    hit(&app, &mut ui, row("shop/api-0"));
    let (said, hits) = texts(&app, &ui);
    assert!(said.iter().any(|one| one == "OPEN · 1"), "{said:?}");
    assert!(hits.rect_of(&Target::ResourceTab(0)).is_some(), "its row");
    for view in [View::Describe, View::Yaml] {
        assert!(
            hits.rect_of(&Target::ResourceView(view)).is_some(),
            "{view:?}"
        );
    }
    assert!(
        said.iter().any(|one| one == "pod · staging · shop"),
        "{said:?}"
    );
    assert!(
        hits.rect_of(&row("shop/api-0")).is_none(),
        "the list gave way"
    );
}

#[test]
fn an_open_row_shows_its_object_and_its_cross_middle_button_or_menu_close_it() {
    let (app, mut ui) = kubed();
    hit(&app, &mut ui, row("shop/api-0"));
    hit(&app, &mut ui, Target::ResourceKind(pods()));
    hit(&app, &mut ui, row("shop/worker-0"));
    assert_eq!(ui.session.resources.showing, Some(1));
    hit(&app, &mut ui, Target::ResourceKind(pods()));
    hit(&app, &mut ui, row("cnpg/cnpg-1"));
    assert_eq!(ui.session.resources.opened.len(), 3);
    hit(&app, &mut ui, Target::ResourceTab(0));
    assert_eq!(ui.session.resources.showing, Some(0));
    ui.hover = Some(Target::ResourceTab(0));
    hit(&app, &mut ui, Target::ResourceClose(0));
    assert_eq!(ui.session.resources.opened.len(), 2);
    let (_, hits) = texts(&app, &ui);
    let open = hits.rect_of(&Target::ResourceTab(0)).expect("its row");
    let (x, y) = (open.x + open.w / 3.0, open.y + open.h / 2.0);
    handle(Input::Middle { x, y }, &mut ui, &app, &hits, window());
    assert_eq!(ui.session.resources.opened.len(), 1);
    handle(Input::Menu { x, y }, &mut ui, &app, &hits, window());
    crate::tests::hit(&Target::MenuRow(0), &mut ui, &app);
    assert!(ui.session.resources.opened.is_empty());
}

#[test]
fn pin_keeps_the_list_under_lists_a_click_brings_it_back_and_pin_or_its_cross_lets_it_go() {
    let (mut app, mut ui) = kubed();
    let asked = hit(&app, &mut ui, Target::ResourcePin);
    let pin = pinned(&mut app, &ui);
    let session = app.session.selected.clone().expect("a session");
    let pinning = session::Command::Pin {
        session: session.clone(),
        pin: pin.clone(),
    };
    assert_eq!(asked, [Command::Session(pinning)]);
    hit(
        &app,
        &mut ui,
        Target::ResourceKind(kind("apps", "Deployment", "deployments", true)),
    );
    ui.session
        .resources
        .hidden_contexts
        .insert("staging".into());
    let (said, _) = texts(&app, &ui);
    assert!(said.iter().any(|one| one == "LISTS · 1"), "{said:?}");
    hit(&app, &mut ui, Target::PinnedList(Box::new(pin.clone())));
    let open = app.session.selected().expect("a session");
    let shown = pins::shown(&app, open, &ui.session.resources);
    assert_eq!(shown.as_ref(), Some(&pin), "its kind and its scope back");
    let unpin = session::Command::Unpin {
        session,
        pin: pin.clone(),
    };
    let asked = hit(&app, &mut ui, Target::ResourcePin);
    assert_eq!(asked, [Command::Session(unpin.clone())]);
    ui.hover = Some(Target::PinnedList(Box::new(pin.clone())));
    let asked = hit(&app, &mut ui, Target::Unpin(Box::new(pin)));
    assert_eq!(asked, [Command::Session(unpin)]);
}

#[test]
fn the_arrows_walk_lists_open_and_kinds_as_drawn_and_left_folds_a_heading() {
    let (mut app, mut ui) = kubed();
    let pin = pinned(&mut app, &ui);
    hit(&app, &mut ui, row("shop/api-0"));
    ui.focus = crate::Focus::Sidebar;
    let (_, hits) = texts(&app, &ui);
    let mut walked = Vec::new();
    for _ in 0..4 {
        let down = Input::Key {
            key: Key::Down,
            mods: Modifiers::default(),
        };
        handle(down, &mut ui, &app, &hits, window());
        walked.push(ui.hover.clone().expect("a row is stood on"));
    }
    assert_eq!(walked[0], Target::PinnedList(Box::new(pin)));
    assert_eq!(walked[1], Target::ResourceTab(0));
    let Target::ResourceGroup(heading) = walked[2].clone() else {
        panic!("a heading after the open rows: {walked:?}");
    };
    assert!(matches!(walked[3], Target::ResourceKind(_)), "{walked:?}");
    ui.hover = Some(walked[2].clone());
    let left = Input::Key {
        key: Key::Left,
        mods: Modifiers::default(),
    };
    handle(left, &mut ui, &app, &hits, window());
    assert!(ui.session.resources.folded.contains(&heading));
}

#[test]
fn alt_o_on_kubernetes_gives_the_middle_the_keys_and_the_arrows_walk_its_rows() {
    let (app, mut ui) = kubed();
    ui.focus = crate::Focus::Rail;
    press(Key::Char('o'), ALT, &mut ui, &app);
    assert_eq!(ui.focus, crate::Focus::Workspace);
    texts(&app, &ui);
    press(Key::Down, Modifiers::default(), &mut ui, &app);
    assert!(
        matches!(ui.hover, Some(Target::ResourceRow(_))),
        "{:?}",
        ui.hover
    );
}

#[test]
fn the_logs_second_row_holds_the_containers_the_run_and_the_range_under_the_band() {
    let (mut app, mut ui) = kubed();
    hit(&app, &mut ui, row("shop/api-0"));
    lands(&mut app, &link("api-0", pods()).key(), vec![crashing()]);
    hit(&app, &mut ui, Target::ResourceView(View::Logs));
    let (_, hits) = texts(&app, &ui);
    let band = hits
        .rect_of(&Target::ResourceView(View::Logs))
        .expect("the band");
    let source = Target::LogSource(LogSource::Container("worker".into()));
    for target in [source, Target::LogPrevious(false), Target::LogRange] {
        let at = hits
            .rect_of(&target)
            .unwrap_or_else(|| panic!("{target:?}"));
        assert!(at.y >= band.bottom(), "{target:?} under the band");
    }
}

#[test]
fn the_wheel_over_the_middle_scrolls_the_list_and_over_the_sidebar_its_rows() {
    let (mut app, mut ui) = kubed();
    let many = (0..60).map(|at| object_row(&format!("api-{at}"), "shop", "Running"));
    let columns = app
        .cluster
        .store
        .watched(&key("staging", Some("shop")))
        .map(|one| one.columns.clone())
        .unwrap_or_default();
    let reset = groove_controllers::cluster_service::Batch::Reset {
        columns,
        rows: many.collect(),
    };
    app.cluster
        .store
        .apply(&key("staging", Some("shop")), reset);
    let mut kinds: Vec<_> = (0..30)
        .map(|at| {
            kind(
                "example.io",
                &format!("Thing{at}"),
                &format!("things{at}"),
                true,
            )
        })
        .collect();
    kinds.push(pods());
    app.cluster.store.set_kinds("staging", kinds);
    let short = crate::tests::metrics(1400, 300, 1.0);
    let (_, hits) = crate::view(&app, &ui, short, &mut Fonts::embedded());
    let layout = crate::layout::Layout::of(short, &ui);
    let delta = Delta::Pixels {
        across: 0.0,
        down: -20.0,
    };
    for (area, which) in [
        (layout.agent, Scroller::Resources),
        (layout.sidebar, Scroller::Kinds),
    ] {
        assert!(hits.extent(which) > 0.0, "{which:?} overflows");
        let (x, y) = (area.x + area.w / 2.0, area.y + area.h / 2.0);
        handle(Input::Scroll { x, y, delta }, &mut ui, &app, &hits, short);
        assert!(ui.offset(which) > 0.0, "{which:?} scrolled");
    }
}
