//! What the language servers found, drawn under the code it covers.

use std::path::Path;

use groove_controllers::AppState;
use groove_controllers::workspace_service::ServerKey;
use groove_gfx::{Fonts, Quad};
use groove_types::{Caret, Diagnostic, DiffView, Severity};
use groove_ui_kit::base::style::{Role, Styles};
use groove_ui_kit::base::tokens::Tokens;

use crate::tests::{full_app, set_face, shows, window};
use crate::views::session::Face;
use crate::{Focus, Ui, view};

pub(super) const PATH: &str = "src/a.py";

/// `a.py` open with `x = None`, a server having found something on columns 4 to 8.
pub(super) fn diagnosed(severity: Severity) -> AppState {
    let mut app = full_app();
    let selected = app.session.selected_worktree().map(|one| one.id.clone());
    let worktree = selected.expect("the fixture has a worktree");
    app.workspace
        .loaded(worktree.clone(), Vec::new(), Default::default());
    shows(&mut app, PATH, "x = 1\n", "x = None\n");
    let key = ServerKey {
        worktree,
        dir: "/w".into(),
        name: "ty".into(),
    };
    let found = Diagnostic {
        start: Caret::new(0, 4),
        end: Caret::new(0, 8),
        severity,
        message: "expected `int`".into(),
        source: Some("ty".into()),
        code: None,
    };
    let at = Path::new("/w").join(PATH);
    app.workspace.servers.diagnosed(&key, &at, vec![found]);
    app
}

/// The steps of every wave drawn in `severity`'s colour, a hairline square each.
fn wave(app: &AppState, face: Face, severity: Severity) -> Vec<Quad> {
    let mut ui = Ui {
        focus: Focus::Workspace,
        ..Ui::default()
    };
    set_face(&mut ui, face);
    let (frame, _) = view(app, &ui, window(), &mut Fonts::embedded());
    let tokens = Tokens::new(1.0);
    let color = Styles::new(app.config.theme(), tokens).fault(severity);
    frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == color && quad.rect.h == tokens.hairline)
        .cloned()
        .collect()
}

fn span(quads: &[Quad]) -> f32 {
    let left = quads.iter().map(|q| q.rect.x).fold(f32::MAX, f32::min);
    let right = quads
        .iter()
        .map(|q| q.rect.right())
        .fold(f32::MIN, f32::max);
    right - left
}

#[test]
fn an_error_runs_a_wavy_line_under_the_characters_it_covers() {
    let app = diagnosed(Severity::Error);
    let quads = wave(&app, Face::File, Severity::Error);
    assert!(
        quads.len() > 4,
        "a wave is many small steps: {}",
        quads.len()
    );
    let advance = Fonts::embedded().advance(Tokens::new(1.0).code);
    let wide = span(&quads);
    assert!(
        (wide - 4.0 * advance).abs() <= 1.0,
        "four characters: {wide}"
    );
    let heights: std::collections::BTreeSet<i32> = quads.iter().map(|q| q.rect.y as i32).collect();
    assert!(heights.len() > 1, "it rises and falls");
}

#[test]
fn a_warning_waves_in_its_own_colour() {
    let app = diagnosed(Severity::Warning);
    let errors = wave(&app, Face::File, Severity::Error);
    let warnings = wave(&app, Face::File, Severity::Warning);
    assert!(errors.is_empty());
    assert!(!warnings.is_empty());
}

#[test]
fn split_marks_only_the_new_side() {
    let app = diagnosed(Severity::Error);
    let file = wave(&app, Face::File, Severity::Error);
    let split = wave(&app, Face::Stream(DiffView::Split), Severity::Error);
    assert!(!split.is_empty(), "the new side shows it");
    assert_eq!(
        split.len(),
        file.len(),
        "one wave: the old side is a past version"
    );
}

#[test]
fn a_file_its_servers_found_nothing_in_draws_no_wave() {
    let mut app = full_app();
    shows(&mut app, PATH, "x = 1\n", "x = None\n");
    let quads = wave(&app, Face::File, Severity::Error);
    assert!(quads.is_empty());
}

/// What the frame draws in `role`'s colour: its icons and its texts.
fn in_role(app: &AppState, face: Face, role: Role) -> (Vec<groove_gfx::Icon>, Vec<String>) {
    let mut ui = Ui {
        focus: Focus::Workspace,
        ..Ui::default()
    };
    set_face(&mut ui, face);
    let (frame, _) = view(
        app,
        &ui,
        crate::tests::metrics(1800, 1000, 1.0),
        &mut Fonts::embedded(),
    );
    let color = Styles::new(app.config.theme(), Tokens::new(1.0)).color(role);
    let layer = &frame.layers()[0];
    let icons = layer
        .icons
        .iter()
        .filter(|one| one.color == color)
        .map(|one| one.icon)
        .collect();
    let texts = layer
        .texts
        .iter()
        .filter(|one| one.style.color == color)
        .map(|one| one.text.clone())
        .collect();
    (icons, texts)
}

#[test]
fn a_file_head_and_its_tree_row_count_its_errors_and_warnings() {
    let mut app = diagnosed(Severity::Error);
    let worktree = app.workspace.worktree.clone().expect("a worktree");
    let key = ServerKey {
        worktree,
        dir: "/w".into(),
        name: "ruff".into(),
    };
    let warned = |line| Diagnostic {
        start: Caret::new(line, 0),
        end: Caret::new(line, 1),
        severity: Severity::Warning,
        message: "unused".into(),
        source: None,
        code: None,
    };
    let at = Path::new("/w").join(PATH);
    app.workspace
        .servers
        .diagnosed(&key, &at, vec![warned(0), warned(0)]);
    let (icons, texts) = in_role(&app, Face::Stream(DiffView::Inline), Role::Warn);
    assert!(icons.contains(&groove_gfx::Icon::Dot), "a warning's mark");
    assert!(
        texts.iter().filter(|one| *one == "2").count() >= 2,
        "on the head and the row: {texts:?}"
    );
    let (icons, texts) = in_role(&app, Face::Stream(DiffView::Inline), Role::Bad);
    assert!(icons.contains(&groove_gfx::Icon::Cross), "an error's mark");
    assert!(texts.iter().any(|one| one == "1"), "{texts:?}");
}
