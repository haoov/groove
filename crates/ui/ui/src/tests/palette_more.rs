//! The palette's reach: the MR, the skills, the discard, and how its input line stands.

use groove_controllers::AppState;

use crate::input::{Key, Modifiers};
use crate::palette::{Palette, entries};
use crate::tests::{full_app, metrics, press, typed_into};
use crate::{Ui, view};

fn grouped(app: &AppState, group: &str) -> Vec<String> {
    let all = entries(app).into_iter();
    all.filter(|one| one.group == group)
        .map(|one| one.label)
        .collect()
}

#[test]
fn the_mr_and_the_skills_have_their_groups() {
    let app = full_app();
    assert_eq!(grouped(&app, "MR"), ["Create the MR"], "no MR yet");
    let open = app.session.selected().expect("a session");
    let sources = groove_controllers::task_service::source_ids(app.config.config.as_ref());
    let offered: Vec<String> = app
        .skill
        .offers(&open.session.kind, &sources)
        .into_iter()
        .map(|one| one.label)
        .collect();
    assert_eq!(
        grouped(&app, "Skills"),
        offered,
        "what the agent's bar offers"
    );
}

#[test]
fn discard_every_change_asks_first() {
    let app = full_app();
    let mut palette = Palette::default();
    typed_into(&mut palette, "discard every", &app);
    let mut ui = Ui::default();
    let commands = ui.closed_palette(palette.key(Key::Enter, &app), &app);
    assert!(commands.is_empty(), "nothing thrown away yet");
    assert!(matches!(
        ui.overlay,
        Some(crate::Overlay::Losing(crate::Losing::Everything))
    ));
}

#[test]
fn the_input_line_stands_taller_than_a_row_and_hints_at_files() {
    let app = full_app();
    let mut ui = Ui::default();
    press(Key::Char('k'), crate::tests::ALT, &mut ui, &app);
    let (frame, _) = view(
        &app,
        &ui,
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let tokens = groove_ui_kit::base::tokens::Tokens::new(1.0);
    let layer = &frame.layers()[1];
    let hint = layer
        .texts
        .iter()
        .find(|t| t.text == "# Files")
        .expect("the hint");
    assert_eq!(hint.height, tokens.bar, "the input line is a bar tall");
    assert!(tokens.bar > tokens.row);
    assert!(
        layer
            .icons
            .iter()
            .any(|one| one.icon == groove_gfx::Icon::Glass)
    );
    press(Key::Char('#'), Modifiers::default(), &mut ui, &app);
    let (frame, _) = view(
        &app,
        &ui,
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    assert!(frame.layers()[1].texts.iter().any(|t| t.text == "Files"));
}

#[test]
fn a_file_found_in_pieces_lights_each_piece_and_the_whole_run_ranks_first() {
    let mut app = full_app();
    let file = |path: &str| groove_types::FileDiff {
        path: path.into(),
        added: 0,
        deleted: 0,
        status: groove_types::FileStatus::Modified,
        staged: None,
    };
    let paths = ["factories/test_keycloak.py", "factories/keycloak.py"].map(file);
    app.workspace.set_paths(paths.to_vec());
    let mut ui = Ui::default();
    press(Key::Char('k'), crate::tests::ALT, &mut ui, &app);
    crate::tests::typed("#factories/keycloak", &mut ui, &app);
    let rows = ui.palette().unwrap().rows(&app);
    assert_eq!(
        rows[0].label, "factories/keycloak.py",
        "the whole run first"
    );
    let (frame, _) = view(
        &app,
        &ui,
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let tokens = groove_ui_kit::base::tokens::Tokens::new(1.0);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), tokens);
    let accent = styles.color(groove_ui_kit::base::style::Role::Accent);
    let lit: Vec<String> = frame.layers()[1]
        .texts
        .iter()
        .filter(|t| t.style.color == accent)
        .map(|t| t.text.clone())
        .collect();
    assert!(
        lit.contains(&"factories/".to_string()) && lit.contains(&"keycloak".to_string()),
        "{lit:?}"
    );
}
