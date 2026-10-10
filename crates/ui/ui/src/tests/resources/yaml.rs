//! An object's YAML, drawn and used as a file is.

use groove_controllers::Command;
use groove_types::Described;

use super::pods;
use super::tab::{crashing, lands, link, opened, runs, wheel};
use crate::Ui;
use crate::hit::Target;
use crate::tests::settings::drawn;
use crate::tests::{click, window};

#[test]
fn the_yaml_view_reads_as_a_file_and_closing_the_tab_brings_the_list_back() {
    let (mut app, mut ui) = opened();
    lands(&mut app, &link("api-0", pods()).key(), vec![crashing()]);
    let (_, hits) = drawn(&app, &ui);
    let yaml = hits
        .rect_of(&Target::ResourceView(true))
        .expect("the yaml view");
    click(yaml, &mut ui, &app, &hits);
    let (texts, hits) = drawn(&app, &ui);
    for shown in ["1", "2", "3", "4"] {
        assert!(
            texts.iter().any(|one| one == shown),
            "line {shown}: {texts:?}"
        );
    }
    let (frame, _) = crate::view(&app, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), window().tokens());
    let runs: Vec<_> = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.clone())
        .collect();
    let key = runs
        .iter()
        .find(|one| one.text.contains("restartCount"))
        .expect("the key drawn");
    let small = styles.small(groove_ui_kit::base::style::Role::Text).size;
    assert!(key.style.size > small, "the YAML keeps the code size");
    assert_ne!(
        key.style.color,
        styles.color(groove_ui_kit::base::style::Role::Text),
        "the key coloured"
    );
    let close = hits
        .rect_of(&Target::ResourceClose(0))
        .expect("the tab's close mark");
    click(close, &mut ui, &app, &hits);
    assert!(ui.session.resources.opened.is_empty());
    let (texts, _) = drawn(&app, &ui);
    assert!(
        texts.iter().any(|one| one.starts_with("Pods · ")),
        "{texts:?}"
    );
}

fn ctrl() -> crate::input::Modifiers {
    crate::input::Modifiers {
        ctrl: true,
        ..Default::default()
    }
}

#[test]
fn in_the_yaml_a_click_lands_the_caret_keys_move_it_and_ctrl_f_finds_and_holds_a_match() {
    use groove_controllers::cluster::Command as Cluster;
    use groove_types::{Caret, Edit, Motion};
    let (mut app, mut ui) = opened();
    let key = link("api-0", pods()).key();
    lands(&mut app, &key, vec![crashing()]);
    let (_, hits) = drawn(&app, &ui);
    click(
        hits.rect_of(&Target::ResourceView(true))
            .expect("the yaml view"),
        &mut ui,
        &app,
        &hits,
    );
    let (_, hits) = drawn(&app, &ui);
    let rows = hits.rect_of(&Target::Code).expect("the yaml rows");
    let line = window().tokens().line;
    let landed = crate::tests::pressed(rows.x + 120.0, rows.y + line * 1.5, &mut ui, &app, &hits);
    let caret = |edit| {
        Command::Cluster(Cluster::Caret {
            key: Box::new(key.clone()),
            edit,
        })
    };
    assert!(
        matches!(
            landed.first(),
            Some(Command::Cluster(Cluster::Caret {
                edit: Edit::Move(Motion::To(_)),
                ..
            }))
        ),
        "{landed:?}"
    );
    assert_eq!(ui.focus, crate::Focus::Workspace);
    let press = |ui: &mut Ui, key, mods| crate::tests::press(key, mods, ui, &app);
    assert_eq!(
        press(&mut ui, crate::input::Key::Down, Default::default()),
        [caret(Edit::Move(Motion::Down))]
    );
    let copy = press(&mut ui, crate::input::Key::Char('c'), ctrl());
    assert_eq!(
        copy,
        [Command::Cluster(Cluster::Copy {
            key: Box::new(key.clone())
        })]
    );
    press(&mut ui, crate::input::Key::Char('f'), ctrl());
    assert!(
        ui.session.resources.tab().is_some()
            && ui.session.find.as_ref().is_some_and(|find| find.typing)
    );
    assert!(
        !ui.session.resources.finding,
        "the list's own search stays shut"
    );
    let mut last = Vec::new();
    for c in "restart".chars() {
        last = press(&mut ui, crate::input::Key::Char(c), Default::default());
    }
    let (from, to) = (Caret::new(3, 2), Caret::new(3, 9));
    assert_eq!(
        last,
        [
            caret(Edit::Move(Motion::To(from))),
            caret(Edit::Extend(Motion::To(to)))
        ]
    );
    press(&mut ui, crate::input::Key::Escape, Default::default());
    assert!(ui.session.resources.tab().is_some() && ui.session.find.is_none());
}

#[test]
fn a_long_yaml_line_scrolls_sideways_by_the_wheel_and_follows_the_caret() {
    let (mut app, mut ui) = opened();
    let key = link("api-0", pods()).key();
    let long = format!(
        "metadata:\n  annotation: {}\nstatus:\n  phase: Running\n",
        "x".repeat(400)
    );
    lands(
        &mut app,
        &key,
        vec![Described {
            yaml: long.into(),
            ..crashing()
        }],
    );
    if let Some(tab) = ui.session.resources.tab_mut() {
        tab.yaml = true;
    }
    ui.focus = crate::Focus::Workspace;
    let metrics = crate::tests::metrics(1920, 1080, 1.0);
    let (_, hits) = crate::view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
    let rows = hits.rect_of(&Target::Code).expect("the yaml rows");
    let key_x = |ui: &Ui| {
        runs(&app, ui)
            .iter()
            .find(|one| one.text.contains("annotation"))
            .map(|one| one.x)
    };
    let before = key_x(&ui);
    wheel(&app, &mut ui, (rows.x + 50.0, rows.y + 50.0), (-200.0, 0.0));
    let across = ui.session.resources.tab().map_or(0.0, |one| one.across);
    assert!(across > 0.0, "the wheel moved the text sideways");
    assert!(key_x(&ui) < before, "the text went left");
    let number = runs(&app, &ui)
        .into_iter()
        .find(|one| one.text == "1")
        .map(|one| one.x);
    assert!(
        number.is_some_and(|x| x < rows.x + 60.0),
        "the numbers stay put"
    );
    let yamls = &mut app.cluster.store.follows.yamls;
    yamls.edit(&key, &groove_types::Edit::Move(groove_types::Motion::Down));
    yamls.edit(
        &key,
        &groove_types::Edit::Move(groove_types::Motion::LineEnd),
    );
    if let Some(tab) = ui.session.resources.tab_mut() {
        tab.across = 0.0;
    }
    let (_, hits) = crate::view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
    crate::input::follow(&mut ui, &app, &hits, metrics);
    let across = ui.session.resources.tab().map_or(0.0, |one| one.across);
    assert!(
        across > 0.0,
        "the caret at the line's end brought it into view"
    );
    let line_start = groove_types::Edit::Move(groove_types::Motion::LineStart);
    app.cluster.store.follows.yamls.edit(&key, &line_start);
    crate::input::follow(&mut ui, &app, &hits, metrics);
    assert_eq!(
        ui.session.resources.tab().map(|one| one.across),
        Some(0.0),
        "and back at its start"
    );
}

#[test]
fn a_search_matching_often_in_a_long_line_draws_at_once() {
    let (mut app, mut ui) = opened();
    let key = link("api-0", pods()).key();
    let applied: String = (0..400)
        .map(|at| format!("{{\"annotations\":{{\"note-{at}\":\"value\"}}}},"))
        .collect();
    let mut pod = crashing();
    pod.yaml = format!(
        "metadata:\n  annotations:\n    kubectl.kubernetes.io/last-applied-configuration: '{applied}'\n  name: api-0\n"
    )
    .into();
    lands(&mut app, &key, vec![pod]);
    let (_, hits) = drawn(&app, &ui);
    let yaml = hits.rect_of(&Target::ResourceView(true)).expect("yaml");
    click(yaml, &mut ui, &app, &hits);
    let press = |ui: &mut Ui, key, mods| crate::tests::press(key, mods, ui, &app);
    press(&mut ui, crate::input::Key::Char('f'), ctrl());
    for c in "annota".chars() {
        press(&mut ui, crate::input::Key::Char(c), Default::default());
    }
    let found = ui.session.find.as_ref().map_or(0, |find| find.hits.len());
    assert!(found > 300, "{found} matches");
    let started = std::time::Instant::now();
    drawn(&app, &ui);
    let took = started.elapsed();
    assert!(took.as_secs() < 5, "a frame took {took:?}");
}
