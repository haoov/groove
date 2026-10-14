//! The palette's look: groups under their headings, an icon on each entry.

use groove_controllers::AppState;

use crate::input::{Key, Modifiers};
use crate::palette::{Palette, entries};
use crate::tests::{app, full_app, metrics, press, typed, typed_into, with_repo};
use crate::{Ui, view};

fn under(query: &str, app: &AppState) -> Vec<(&'static str, String)> {
    let mut palette = Palette::default();
    typed_into(&mut palette, query, app);
    let rows = palette.rows(app).into_iter();
    rows.map(|e| (e.group, e.label)).collect()
}

#[test]
fn a_group_named_first_comes_first_and_whole() {
    let app = with_repo(app());
    let all = under("", &app);
    let go = all.iter().filter(|(group, _)| *group == "Go").count();
    let typed = under("go", &app);
    assert!(
        typed[..go].iter().all(|(group, _)| *group == "Go"),
        "{typed:?}"
    );
    let pushed = under("git pu", &app);
    assert_eq!(pushed[0], ("Git", "Push".to_string()));
    assert_eq!(under("push", &app)[0], ("Git", "Push".to_string()));
}

#[test]
fn the_query_keeps_each_group_in_one_run() {
    let app = with_repo(app());
    for query in ["", "e", "the", "show"] {
        let rows = under(query, &app);
        let mut seen: Vec<&str> = Vec::new();
        for (group, _) in &rows {
            if seen.last() != Some(group) {
                assert!(!seen.contains(group), "{query:?} splits {group}: {rows:?}");
                seen.push(group);
            }
        }
    }
}

#[test]
fn the_selected_row_shows_under_its_heading() {
    let app = with_repo(app());
    let mut ui = Ui::default();
    press(Key::Char('k'), crate::tests::ALT, &mut ui, &app);
    let rows = under("", &app);
    let at = rows
        .iter()
        .position(|(group, _)| *group == "Notes")
        .expect("notes");
    for _ in 0..at {
        press(Key::Down, Modifiers::default(), &mut ui, &app);
    }
    let (frame, _) = view(
        &app,
        &ui,
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts: Vec<String> = frame.layers()[1]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.contains(&"NOTES".to_string()), "{texts:?}");
    assert!(texts.contains(&rows[at].1), "{texts:?}");
    typed("pu", &mut ui, &app);
    let (frame, _) = view(
        &app,
        &ui,
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts: Vec<String> = frame.layers()[1]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(
        texts.contains(&"GIT".to_string()),
        "headings while typing too: {texts:?}"
    );
}

#[test]
fn every_entry_draws_an_icon() {
    for app in [AppState::default(), app(), with_repo(app())] {
        for entry in entries(&app) {
            let mark = crate::palette::mark_of(&entry, &app);
            assert!(mark.is_some(), "{} has no icon", entry.label);
        }
    }
}

#[test]
fn the_selected_row_lights_its_icon() {
    let app = full_app();
    let mut ui = Ui::default();
    press(Key::Char('k'), crate::tests::ALT, &mut ui, &app);
    let (frame, _) = view(
        &app,
        &ui,
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let styles = groove_ui_kit::base::style::Styles::new(
        app.config.theme(),
        groove_ui_kit::base::tokens::Tokens::new(1.0),
    );
    let color = |icon: groove_gfx::Icon| {
        let drawn = frame.layers()[1].icons.iter().find(|one| one.icon == icon);
        drawn.map(|one| one.color).expect("drawn")
    };
    use groove_ui_kit::base::style::Role;
    assert_eq!(
        color(groove_gfx::Icon::Kanban),
        styles.color(Role::Text),
        "Board, selected"
    );
    assert_eq!(
        color(groove_gfx::Icon::Eye),
        styles.color(Role::Faint),
        "Overview"
    );
}
