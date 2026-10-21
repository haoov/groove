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

/// The texts the palette draws in the accent colour, after `query` is typed.
fn lit_under(query: &str) -> Vec<String> {
    let app = full_app();
    let mut ui = Ui::default();
    press(Key::Char('k'), crate::tests::ALT, &mut ui, &app);
    typed(query, &mut ui, &app);
    let (frame, _) = view(
        &app,
        &ui,
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let tokens = groove_ui_kit::base::tokens::Tokens::new(1.0);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), tokens);
    let accent = styles.color(groove_ui_kit::base::style::Role::Accent);
    let layer = &frame.layers()[1];
    let lit: Vec<_> = layer
        .texts
        .iter()
        .filter(|t| t.style.color == accent)
        .collect();
    for text in &lit {
        let under = layer
            .quads
            .iter()
            .any(|q| q.color == accent && q.rect.x == text.x);
        assert!(under, "{} is underlined", text.text);
    }
    lit.iter().map(|t| t.text.clone()).collect()
}

#[test]
fn the_letters_the_query_matched_are_lit() {
    assert!(lit_under("").is_empty(), "nothing typed, nothing lit");
    assert!(lit_under("pu").contains(&"Pu".to_string()));
    let named = lit_under("git pu");
    assert!(
        named.contains(&"Pu".to_string()) && !named.iter().any(|t| t.to_lowercase() == "git"),
        "{named:?}"
    );
    let both = lit_under("show not");
    assert!(
        both.contains(&"Show".to_string()) && both.contains(&"not".to_string()),
        "{both:?}"
    );
}

fn drawn(ui: &Ui, app: &AppState) -> groove_gfx::Frame {
    view(
        app,
        ui,
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    )
    .0
}

#[test]
fn the_palette_stands_as_glass_with_no_shadow() {
    let app = full_app();
    let mut ui = Ui::default();
    press(Key::Char('k'), crate::tests::ALT, &mut ui, &app);
    let frame = drawn(&ui, &app);
    let top = &frame.layers()[1];
    assert_eq!(top.glass.len(), 1);
    assert!(!top.quads.iter().any(|q| q.shape.blur > 0.0));
}

#[test]
fn a_typed_step_hints_at_enter_and_an_empty_list_says_why() {
    let app = full_app();
    let mut ui = Ui::default();
    press(Key::Char('k'), crate::tests::ALT, &mut ui, &app);
    typed("zzz", &mut ui, &app);
    let texts = |frame: &groove_gfx::Frame| -> Vec<String> {
        frame.layers()[1]
            .texts
            .iter()
            .map(|t| t.text.clone())
            .collect()
    };
    let none = texts(&drawn(&ui, &app));
    assert!(
        none.contains(&"No match for \u{201c}zzz\u{201d}".to_string()),
        "{none:?}"
    );
    let mut ui = Ui::default();
    press(Key::Char('k'), crate::tests::ALT, &mut ui, &app);
    typed("add work", &mut ui, &app);
    press(Key::Enter, Modifiers::default(), &mut ui, &app);
    press(Key::Enter, Modifiers::default(), &mut ui, &app);
    let branch = texts(&drawn(&ui, &app));
    assert_eq!(
        branch,
        ["branch", "Enter to confirm"],
        "the hint in the input line, no list"
    );
}

#[test]
fn the_last_row_selected_keeps_the_inset_under_it() {
    let app = full_app();
    let mut ui = Ui::default();
    press(Key::Char('k'), crate::tests::ALT, &mut ui, &app);
    for _ in 0..under("", &app).len() {
        press(Key::Down, Modifiers::default(), &mut ui, &app);
    }
    let (frame, hits) = view(
        &app,
        &ui,
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let tokens = groove_ui_kit::base::tokens::Tokens::new(1.0);
    let chosen = groove_ui_kit::base::style::Styles::new(app.config.theme(), tokens).glass_chosen();
    let boxed = hits
        .rect_of(&crate::hit::Target::Palette)
        .expect("the palette");
    let mut quads = frame.layers()[1].quads.iter();
    let row = quads
        .rfind(|q| q.color == chosen)
        .expect("the selected row");
    assert_eq!(
        row.rect.bottom(),
        boxed.bottom() - tokens.sm,
        "the inset, no more"
    );
}

#[test]
fn a_pinned_line_names_what_the_actions_run_on() {
    let app = full_app();
    let open = app.session.selected().expect("a session");
    let branch = open.selected_worktree().expect("a worktree").branch.clone();
    let mut ui = Ui::default();
    press(Key::Char('k'), crate::tests::ALT, &mut ui, &app);
    let texts = |ui: &Ui| -> Vec<String> {
        let frame = drawn(ui, &app);
        frame.layers()[1]
            .texts
            .iter()
            .map(|t| t.text.clone())
            .collect()
    };
    let shown = texts(&ui);
    assert!(shown.contains(&"in ".to_string()), "{shown:?}");
    assert!(
        shown.contains(&open.session.title) && shown.contains(&branch),
        "{shown:?}"
    );
    typed("add work", &mut ui, &app);
    press(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert!(
        !texts(&ui).contains(&"in ".to_string()),
        "a step asks its own question"
    );
    let none = AppState::default();
    let mut bare = Ui::default();
    press(Key::Char('k'), crate::tests::ALT, &mut bare, &none);
    let frame = drawn(&bare, &none);
    assert!(
        !frame.layers()[1].texts.iter().any(|t| t.text == "in "),
        "no session, no scope"
    );
}

#[test]
fn a_hash_searches_the_worktree_files_and_opens_the_one_picked() {
    let mut app = full_app();
    let file = |path: &str| groove_types::FileDiff {
        path: path.into(),
        added: 0,
        deleted: 0,
        status: groove_types::FileStatus::Modified,
        staged: None,
    };
    let paths = ["src/play.rs", "docs/layout.md", "src/layout.rs"]
        .map(file)
        .to_vec();
    app.workspace.set_paths(paths);
    let mut palette = Palette::default();
    let listed = palette.key(Key::Char('#'), &app);
    assert_eq!(listed.commands[0].id(), "workspace.list_paths");
    typed_into(&mut palette, "layout", &app);
    let rows: Vec<String> = palette.rows(&app).into_iter().map(|e| e.label).collect();
    assert_eq!(
        rows,
        ["src/layout.rs", "docs/layout.md"],
        "names first, short first"
    );
    let mut ui = Ui::default();
    let commands = ui.closed_palette(palette.key(Key::Enter, &app), &app);
    assert_eq!(commands[0].id(), "workspace.open_file");
    assert_eq!(ui.session.tab, crate::views::session::Tab::Files);
}
