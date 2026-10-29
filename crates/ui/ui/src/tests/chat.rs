//! The agent's conversation in its pane: drawn from the transcript, answered from the prompt, linked to the diff.

use std::path::PathBuf;

use groove_controllers::agent_service::{Event, apply};
use groove_controllers::{AppState, Command, agent, session, workspace};
use groove_gfx::Fonts;
use groove_types::chat::{Heard, Outcome, Patch};
use groove_types::{SessionId, WorktreeId};

use crate::hit::Target;
use crate::input::{Delta, Input, handle};
use crate::input::{Key, Modifiers};
use crate::tests::{
    all_texts, changed_files, click, drag_at, press, pressed, release, typed, window, with_repo,
};
use crate::views::session::Tab;
use crate::{Focus, Surface, Ui};

mod reading;

const WORKTREE: &str = "/code/worktrees/a/kiwi/explorer/alpha";

/// Session `a` with a running agent whose transcript said `heard`.
pub(super) fn talking(heard: Vec<Heard>) -> AppState {
    let mut app = with_repo(super::bar::asking(Vec::new()));
    app.agent.agents[0].1.terminal = Some(super::agent::mouse_reader());
    let (session, transcript) = (SessionId::new("a"), PathBuf::from("/t/a.jsonl"));
    app.agent.follow(&session, transcript.clone());
    let event = Event::Heard {
        session,
        transcript,
        heard,
    };
    apply(&mut app.agent, event);
    app
}

pub(super) fn ran(command: &str) -> Heard {
    Heard::Ran {
        id: command.into(),
        command: command.into(),
        about: None,
    }
}

pub(super) fn chat_ui() -> Ui {
    Ui {
        surface: Surface::Session,
        focus: Focus::Agent,
        ..Ui::default()
    }
}

fn edit(path: &str, line: u32) -> Vec<Heard> {
    let patch = Patch {
        added: 2,
        removed: 1,
        line,
    };
    vec![
        Heard::Edit {
            id: "e".into(),
            path: path.into(),
            created: false,
        },
        Heard::Result {
            id: "e".into(),
            outcome: Outcome {
                patch: Some(patch),
                ..Outcome::default()
            },
        },
    ]
}

#[test]
fn the_conversation_shows_in_place_of_the_terminal_and_the_bar_brings_the_terminal_back() {
    let app = talking(vec![
        Heard::You("cache the quotes".into()),
        Heard::Clock(groove_types::Timestamp::new(100)),
        Heard::Thought,
        ran("cargo build"),
        ran("cargo test"),
        Heard::Clock(groove_types::Timestamp::new(126)),
        Heard::Queued("then commit".into()),
    ]);
    let mut ui = chat_ui();
    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let texts = all_texts(&frame);
    let shown = [
        "cache the quotes",
        "bash 2",
        "cargo build",
        "cargo test",
        "26s",
        "then commit",
        "terminal",
    ];
    for said in shown {
        assert!(texts.iter().any(|one| one == said), "{said}: {texts:?}");
    }
    let toggle = hits.rect_of(&Target::AgentView).expect("the bar's toggle");
    assert!(click(toggle, &mut ui, &app, &hits).is_empty());
    assert!(ui.agent.terminal, "the terminal shows");
    let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let texts = all_texts(&frame);
    assert!(!texts.iter().any(|one| one == "cache the quotes"));
    assert!(texts.iter().any(|one| one == "conversation"), "{texts:?}");
}

#[test]
fn enter_sends_the_prompt_to_the_agent_and_shift_enter_breaks_its_line() {
    let app = talking(Vec::new());
    let mut ui = chat_ui();
    typed("look", &mut ui, &app);
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    assert!(press(Key::Enter, shift, &mut ui, &app).is_empty());
    typed("again", &mut ui, &app);
    let sent = press(Key::Enter, Modifiers::default(), &mut ui, &app);
    let say = agent::Command::Say {
        session: SessionId::new("a"),
        text: "look\nagain".into(),
    };
    assert_eq!(sent, vec![Command::Agent(say)]);
    assert!(ui.agent.prompt.is_empty(), "the prompt empties once sent");
}

#[test]
fn an_empty_prompt_sends_nothing() {
    let app = talking(Vec::new());
    let mut ui = chat_ui();
    typed("   ", &mut ui, &app);
    assert!(press(Key::Enter, Modifiers::default(), &mut ui, &app).is_empty());
}

#[test]
fn a_click_on_an_edit_s_file_opens_its_hunk_in_the_diff() {
    let mut app = talking(edit(&format!("{WORKTREE}/src/lib.rs"), 30));
    let before: String = (1..=40).map(|n| format!("line {n}\n")).collect();
    let after = before.replace("line 30\n", "line thirty\nline 30b\n");
    changed_files(&mut app, &[("src/lib.rs", &before, &after)]);
    let mut ui = chat_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let link = Target::Edited {
        path: format!("{WORKTREE}/src/lib.rs"),
        line: 30,
    };
    let rect = hits.rect_of(&link).expect("the edit's file link");
    assert!(
        click(rect, &mut ui, &app, &hits).is_empty(),
        "the worktree is already selected"
    );
    assert_eq!(ui.session.tab, Tab::Diff);

    crate::input::reveal(&mut ui, &app, window());
    let row = app
        .workspace
        .changes
        .row_of("src/lib.rs", 30)
        .expect("the hunk's row");
    let line = window().tokens().code_line;
    assert!(ui.session.diff > 0.0 && ui.session.diff <= row as f32 * line);
    assert!(ui.session.revealing.is_none(), "it scrolls once");
}

#[test]
fn an_edit_in_another_worktree_selects_it_first() {
    let mut app = talking(edit(&format!("{WORKTREE}/src/lib.rs"), 1));
    app.session
        .get_mut(&SessionId::new("a"))
        .unwrap()
        .state
        .selected_worktree = None;
    let mut ui = chat_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let link = Target::Edited {
        path: format!("{WORKTREE}/src/lib.rs"),
        line: 1,
    };
    let rect = hits.rect_of(&link).expect("the edit's file link");
    let select = session::Command::SelectWorktree {
        session: SessionId::new("a"),
        worktree: WorktreeId::new("wt-1"),
    };
    assert_eq!(
        click(rect, &mut ui, &app, &hits),
        vec![Command::Session(select)]
    );
    assert_eq!(ui.session.revealing, Some(("src/lib.rs".into(), 1)));
}

pub(super) const CTRL: Modifiers = Modifiers {
    ctrl: true,
    alt: false,
    shift: false,
};

pub(super) fn copied(text: &str) -> Vec<Command> {
    let text = text.to_string();
    vec![Command::Workspace(workspace::Command::CopyText { text })]
}

#[test]
fn a_drag_over_the_conversation_copies_what_it_holds() {
    let app = talking(vec![Heard::You("cache the quotes".into())]);
    let mut ui = chat_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let run = hits
        .read()
        .iter()
        .find(|one| one.text == "cache the quotes");
    let rect = run.expect("the bubble's words").rect;
    let y = rect.y + rect.h / 2.0;
    pressed(rect.x, y, &mut ui, &app, &hits);
    drag_at(rect.right() + 1.0, y, &mut ui, &app, &hits);
    assert_eq!(release(&mut ui, &app, &hits), copied("cache the quotes"));
}

#[test]
fn the_prompt_keeps_its_spaces_and_selects_and_copies_as_a_note_does() {
    let app = talking(Vec::new());
    let mut ui = chat_ui();
    typed("one two ", &mut ui, &app);
    assert_eq!(ui.agent.prompt.text(), "one two ");
    press(Key::Char('a'), CTRL, &mut ui, &app);
    assert_eq!(
        press(Key::Char('c'), CTRL, &mut ui, &app),
        copied("one two ")
    );

    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let (row, _) = hits.typed().rows[0];
    let y = row.y + row.h / 2.0;
    pressed(row.x, y, &mut ui, &app, &hits);
    drag_at(row.x + window().advance * 3.0, y, &mut ui, &app, &hits);
    release(&mut ui, &app, &hits);
    assert_eq!(press(Key::Char('c'), CTRL, &mut ui, &app), copied("one"));
}

#[test]
fn the_prompt_follows_its_caret_past_its_tallest_and_the_wheel_scrolls_it() {
    let app = talking(Vec::new());
    let mut ui = chat_ui();
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    for _ in 0..12 {
        typed("line", &mut ui, &app);
        press(Key::Enter, shift, &mut ui, &app);
    }
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let followed = hits.typed().scroll;
    assert!(followed > 0.0, "the caret's row shows");
    let (row, _) = hits.typed().rows[0];
    let up = Input::Scroll {
        x: row.x,
        y: row.y,
        delta: Delta::Lines {
            across: 0.0,
            down: 3.0,
        },
    };
    handle(up, &mut ui, &app, &hits, window());
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(hits.typed().scroll < followed, "the wheel moves the rows");
}
