//! The popup: asked once the pointer rests on a spot, or from the caret on `ctrl+k`.

use groove_controllers::{AppState, Command, workspace};
use groove_gfx::Fonts;
use groove_types::Caret;

use super::diagnostics::{PATH, diagnosed};
use crate::input::{Key, Modifiers, hovering};
use crate::tests::{all_texts, metrics, press, set_face};
use crate::views::session::Face;
use crate::views::session::diff::Pointed;
use crate::{Focus, Ui, view};

const ON: Caret = Caret { line: 0, column: 5 };

fn pointing_at(at: Caret) -> Ui {
    let mut ui = Ui {
        focus: Focus::Workspace,
        ..Ui::default()
    };
    set_face(&mut ui, Face::File);
    ui.session.pointed = Some(Pointed {
        path: PATH.into(),
        at,
    });
    ui
}

/// What the frame says at `tick`, every layer.
fn said(app: &AppState, ui: &Ui, tick: u64) -> String {
    let mut at = metrics(1600, 1000, 1.0);
    at.tick = tick;
    let (frame, _) = view(app, ui, at, &mut Fonts::embedded());
    all_texts(&frame).join("\n")
}

#[test]
fn the_pointer_asks_once_it_has_rested_and_never_twice() {
    let mut app = diagnosed(groove_types::Severity::Error);
    let mut ui = pointing_at(ON);
    assert_eq!(hovering(&mut ui, &app, 1000), None, "it comes to rest");
    assert_eq!(hovering(&mut ui, &app, 1100), None, "not yet");
    let asked = hovering(&mut ui, &app, 1250);
    let hover = Command::Workspace(workspace::Command::Hover {
        path: PATH.into(),
        at: ON,
        caret: false,
    });
    assert_eq!(asked, Some(hover));
    app.workspace.servers.asked(PATH, ON, false);
    assert_eq!(hovering(&mut ui, &app, 1400), None, "asked already");
}

#[test]
fn a_rested_pointer_shows_the_diagnostic_over_what_the_hover_said() {
    let mut app = diagnosed(groove_types::Severity::Error);
    app.workspace.servers.asked(PATH, ON, false);
    let markdown = "```python\nNone\n```\nThe absence of a value.";
    app.workspace
        .servers
        .answered(PATH, ON, Some(markdown.into()));
    let mut ui = pointing_at(ON);
    ui.session.pointer_rest = ui.session.pointed.clone().map(|at| (at, 0));
    let shown = said(&app, &ui, 300);
    let message = shown.find("expected ").expect("the diagnostic");
    let hover = shown.find("The absence of a value.").expect("the hover");
    assert!(message < hover, "the diagnostic stands above: {shown}");
    assert!(shown.contains("None"), "the code block, its fence dropped");
    assert!(!shown.contains("```"), "{shown}");
    assert!(shown.contains("ty"), "who found it");
}

#[test]
fn the_popup_waits_for_the_rest_and_leaves_with_the_pointer() {
    let mut app = diagnosed(groove_types::Severity::Error);
    app.workspace.servers.asked(PATH, ON, false);
    let mut ui = pointing_at(ON);
    ui.session.pointer_rest = ui.session.pointed.clone().map(|at| (at, 1000));
    assert!(!said(&app, &ui, 1100).contains("expected "), "too soon");
    assert!(said(&app, &ui, 1300).contains("expected "));
    ui.session.pointed = Some(Pointed {
        path: PATH.into(),
        at: Caret::new(0, 0),
    });
    assert!(!said(&app, &ui, 1400).contains("expected "), "moved off");
}

#[test]
fn ctrl_k_asks_at_the_caret() {
    let app = diagnosed(groove_types::Severity::Error);
    let mut ui = Ui {
        focus: Focus::Workspace,
        ..Ui::default()
    };
    set_face(&mut ui, Face::File);
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };
    let asked = press(Key::Char('k'), ctrl, &mut ui, &app);
    let caret = app.workspace.active().expect("open").new.caret();
    let hover = Command::Workspace(workspace::Command::Hover {
        path: PATH.into(),
        at: caret,
        caret: true,
    });
    assert_eq!(asked, vec![hover]);
}

#[test]
fn a_popup_asked_at_the_caret_stands_while_the_caret_does() {
    let mut app = diagnosed(groove_types::Severity::Error);
    let caret = app.workspace.active().expect("open").new.caret();
    app.workspace
        .servers
        .answered(PATH, caret, Some("unused".into()));
    app.workspace.servers.asked(PATH, caret, true);
    app.workspace
        .servers
        .answered(PATH, caret, Some("x: int".into()));
    let mut ui = Ui {
        focus: Focus::Workspace,
        ..Ui::default()
    };
    set_face(&mut ui, Face::File);
    assert!(said(&app, &ui, 0).contains("x: int"), "at once, no rest");
    ui.focus = Focus::Rail;
    assert!(!said(&app, &ui, 0).contains("x: int"), "the keyboard left");
}

/// The popup's texts with the colour each is drawn in, the pointer rested on `ON`.
fn runs(app: &AppState) -> Vec<(String, groove_gfx::Color, f32)> {
    let mut ui = pointing_at(ON);
    ui.session.pointer_rest = ui.session.pointed.clone().map(|at| (at, 0));
    let mut at = metrics(1600, 1000, 1.0);
    at.tick = 300;
    let (frame, _) = view(app, &ui, at, &mut Fonts::embedded());
    let last = frame.layers().last().expect("the popup's layer");
    last.texts
        .iter()
        .map(|one| (one.text.clone(), one.style.color, one.y))
        .collect()
}

fn styles(app: &AppState) -> groove_ui_kit::base::style::Styles {
    let tokens = groove_ui_kit::base::tokens::Tokens::new(1.0);
    groove_ui_kit::base::style::Styles::new(app.config.theme(), tokens)
}

#[test]
fn the_hover_s_code_is_coloured_as_the_file_is_and_its_inline_code_set_apart() {
    let mut app = diagnosed(groove_types::Severity::Error);
    app.workspace.servers.asked(PATH, ON, false);
    let said = "```python\ndef put(self) -> None\n```\nKeeps `quote`.";
    app.workspace.servers.answered(PATH, ON, Some(said.into()));
    let runs = runs(&app);
    let keyword = styles(&app).syntax(groove_types::Capture::Keyword);
    assert!(
        runs.iter()
            .any(|(text, color, _)| text == "def" && *color == keyword),
        "{runs:?}"
    );
    assert!(
        runs.iter().any(|(text, ..)| text == "quote"),
        "the inline code, its ticks dropped"
    );
    assert!(!runs.iter().any(|(text, ..)| text.contains('`')));
    assert!(
        runs.iter()
            .any(|(text, ..)| text == "ctrl+click: go to definition"),
        "the footer"
    );
}

#[test]
fn a_long_message_wraps_by_words_inside_the_panel() {
    let mut app = diagnosed(groove_types::Severity::Error);
    let long = "the cache keeps one quote a route ".repeat(12);
    let worktree = app.workspace.worktree.clone().expect("a worktree");
    let key = groove_controllers::workspace_service::ServerKey {
        worktree,
        dir: "/w".into(),
        name: "ty".into(),
    };
    let found = groove_types::Diagnostic {
        start: ON,
        end: ON,
        severity: groove_types::Severity::Warning,
        message: long.clone(),
        source: None,
        code: None,
    };
    let at = std::path::Path::new("/w").join(PATH);
    app.workspace.servers.diagnosed(&key, &at, vec![found]);
    app.workspace.servers.asked(PATH, ON, false);
    let warn = styles(&app).color(groove_ui_kit::base::style::Role::Warn);
    let rows: Vec<(String, f32)> = runs(&app)
        .into_iter()
        .filter(|(_, color, _)| *color == warn)
        .map(|(text, _, y)| (text, y))
        .collect();
    assert!(rows.len() > 1, "more than one row: {rows:?}");
    assert!(
        rows.iter()
            .all(|(text, _)| !text.starts_with(' ')
                && text.split(' ').all(|word| long.contains(word))),
        "cut at spaces: {rows:?}"
    );
}

#[test]
fn a_diagnostic_s_title_takes_its_colour_and_its_details_and_source_follow_plainly() {
    let mut app = diagnosed(groove_types::Severity::Error);
    let worktree = app.workspace.worktree.clone().expect("a worktree");
    let key = groove_controllers::workspace_service::ServerKey {
        worktree,
        dir: "/w".into(),
        name: "ty".into(),
    };
    let message = "Cannot resolve imported module `pythie`\ninfo: Searched in the following paths\ninfo: Make sure your environment is configured";
    let found = groove_types::Diagnostic {
        start: ON,
        end: ON,
        severity: groove_types::Severity::Error,
        message: message.into(),
        source: Some("ty".into()),
        code: Some("unresolved-import".into()),
    };
    let at = std::path::Path::new("/w").join(PATH);
    app.workspace.servers.diagnosed(&key, &at, vec![found]);
    app.workspace.servers.asked(PATH, ON, false);
    let runs = runs(&app);
    let styles = styles(&app);
    let color = |role| styles.color(role);
    let of = |text: &str| runs.iter().find(|(one, ..)| one.starts_with(text)).cloned();
    let title = of("Cannot resolve").expect("the title");
    let detail = of("info: Searched").expect("a detail, on its own line");
    let source = of("ty · unresolved-import").expect("who found it");
    use groove_ui_kit::base::style::Role;
    assert_eq!(title.1, color(Role::Bad));
    assert_ne!(detail.1, color(Role::Bad), "only the title is red");
    assert!(
        title.2 < detail.2 && detail.2 < source.2,
        "title, details, then source: {runs:?}"
    );
}

#[test]
fn a_code_block_stands_on_its_own_ground_in_the_panel() {
    let mut app = diagnosed(groove_types::Severity::Error);
    app.workspace.servers.asked(PATH, ON, false);
    let said = "```python\ndef put(self) -> None\n```";
    app.workspace.servers.answered(PATH, ON, Some(said.into()));
    let mut ui = pointing_at(ON);
    ui.session.pointer_rest = ui.session.pointed.clone().map(|at| (at, 0));
    let mut at = metrics(1600, 1000, 1.0);
    at.tick = 300;
    let (frame, _) = view(&app, &ui, at, &mut Fonts::embedded());
    let styles = styles(&app);
    let last = frame.layers().last().expect("the popup's layer");
    let ground = styles.prose_code_ground();
    assert_ne!(ground, styles.band(), "not the panel's own");
    assert!(last.quads.iter().any(|quad| quad.color == ground));
}
