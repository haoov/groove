//! The agent's own row: the toggle between its terminal and its conversation.

use groove_controllers::agent_service::Agent;
use groove_gfx::Fonts;
use groove_types::{AgentStatus, ApprovalId, Ask, SessionActivity, SessionId, Timestamp};

use crate::tests::{app, render, window};
use crate::{Surface, Ui};

mod converting;

/// The fixture's session, with the writes it waits on.
pub(super) fn asking(asks: Vec<Ask>) -> groove_controllers::AppState {
    let mut app = app();
    let activity = SessionActivity {
        status: AgentStatus::Idle,
        tool: None,
        asks,
        changed_at: Timestamp::new(0),
        seen_at: None,
    };

    app.agent.agents.push((
        SessionId::new("a"),
        Agent {
            terminal: None,
            stream: None,
            permissions: Vec::new(),
            activity,
            started_at: Timestamp::new(0),
            launch: 0,
        },
    ));
    app
}

pub(super) fn ask(id: &str, op: &str, subject: &str) -> Ask {
    Ask {
        id: ApprovalId::new(id),
        op: op.to_string(),
        subject: subject.to_string(),
        text: subject.to_string(),
        worktree: None,
    }
}

fn drawn(app: &groove_controllers::AppState, ui: &Ui) -> Vec<String> {
    render(app, ui, window()).0
}

pub(super) fn session_ui() -> Ui {
    Ui {
        surface: Surface::Session,
        ..Ui::default()
    }
}

/// The fixture's session, with the skills it can be sent.
fn offering(skills: Vec<groove_types::Skill>) -> groove_controllers::AppState {
    let mut app = asking(Vec::new());
    app.skill.skills = skills;
    app
}

fn skill(name: &str, label: &str, kinds: &[&str]) -> groove_types::Skill {
    groove_types::Skill {
        id: format!("groove:{name}"),
        plugin: "groove".into(),
        name: name.into(),
        description: "What it does.".into(),
        hint: String::new(),
        label: label.into(),
        kinds: kinds.iter().map(|one| one.to_string()).collect(),
        resources: Vec::new(),
        editable: false,
        enabled: true,
        changed_at: Timestamp::new(0),
    }
}

#[test]
fn the_bar_stands_empty_on_the_screen_s_ground() {
    let app = offering(vec![skill("co-review", "co review", &[])]);
    let ui = session_ui();
    let drawn = drawn(&app, &ui);
    for gone in [
        "skills",
        "reload",
        "1 skill",
        "auto-approve",
        "conversation",
    ] {
        assert!(!drawn.iter().any(|one| one == gone), "{gone}: {drawn:?}");
    }
    let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let bar = crate::layout::Layout::of(window(), &ui).agent_bar;
    let base = &frame.layers()[0];
    let pills = base
        .glass
        .iter()
        .filter(|one| bar.contains(one.rect.x, one.rect.y + 1.0));
    assert_eq!(pills.count(), 0, "no pill of glass");
    let deep =
        groove_ui_kit::base::style::Styles::new(app.config.theme(), crate::Tokens::new(1.0)).deep();
    let pane = crate::layout::Layout::of(window(), &ui).agent;
    assert!(
        pane.contains(bar.x, bar.y),
        "the bar stands in the agent's pane"
    );
    let quads = base.quads.iter().filter(|one| one.rect == pane);
    assert_eq!(quads.count(), 0, "no fill over the glass");
    let board = crate::layout::Layout::of(window(), &ui).board;
    assert!(
        base.under.contains(&(board, deep)),
        "on the screen's own ground"
    );
}

#[test]
fn the_row_stands_under_the_screen_and_the_grid_gives_way() {
    let ui = session_ui();
    let layout = crate::layout::Layout::of(window(), &ui);
    let tokens = crate::Tokens::new(1.0);
    assert_eq!(
        layout.agent_bar.bottom(),
        layout.agent.bottom(),
        "it sits at the pane's foot"
    );
    assert_eq!(
        layout.agent_bar.h,
        tokens.bar + tokens.sm,
        "a bar and its gap"
    );
    assert_eq!(
        layout.agent_origin(&tokens).1,
        layout.agent.y + tokens.sm,
        "the screen starts at the top as it did"
    );
}

#[test]
fn the_wheel_over_the_agent_reaches_its_own_screen() {
    let app = offering(Vec::new());
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let pane = crate::layout::Layout::of(window(), &ui).agent;
    let acted = crate::input::handle(
        crate::input::Input::Scroll {
            x: pane.x + pane.w / 2.0,
            y: pane.y + pane.h / 2.0,
            delta: crate::input::Delta::Lines {
                across: 0.0,
                down: 3.0,
            },
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    let said: Vec<&str> = acted.iter().map(|one| one.id()).collect();
    assert_eq!(said, ["agent.scroll"], "the wheel is the agent's own");
}

/// The config of a machine whose task sources are set up.
pub(super) fn sourced(
    mut app: groove_controllers::AppState,
    github: bool,
    notion: bool,
) -> groove_controllers::AppState {
    let git = r#"{ "git": { "worktree_root": "code/worktrees" } }"#;
    let mut config: groove_types::Config = serde_json::from_str(git).unwrap();
    config.notion = notion.then(|| groove_types::NotionConfig {
        token: "t".into(),
        database_id: "d".into(),
        user_id: "u".into(),
        assignee: Some("Assignee".into()),
        sprint: Some("Sprint".into()),
        sprint_status: None,
        properties: Default::default(),
        status_map: Default::default(),
        priority_map: Default::default(),
        filters: groove_types::FilterConfig {
            exclude_statuses: Vec::new(),
        },
        task_template_page_id: None,
        default_project_id: None,
    });
    config.github = github.then(|| groove_types::GithubConfig {
        host: "github.com".into(),
        token: None,
        properties: Default::default(),
        status_map: Default::default(),
        priority_map: Default::default(),
    });
    app.config.config = Some(config);
    app
}

/// Where the pointer lands in the middle of the agent's own screen.
fn on_screen(ui: &Ui) -> (f32, f32) {
    let pane = crate::layout::Layout::of(window(), ui).agent;
    (pane.x + pane.w / 2.0, pane.y + pane.h / 3.0)
}

#[test]
fn a_press_on_the_screen_begins_a_selection() {
    let app = offering(Vec::new());
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let (x, y) = on_screen(&ui);
    let acted = crate::input::handle(
        crate::input::Input::Press {
            x,
            y,
            mods: Default::default(),
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert!(
        matches!(ui.held, Some(crate::Held::Selecting(crate::Grid::Agent))),
        "the pointer is choosing what to hold"
    );
    let said: Vec<&str> = acted.iter().map(|one| one.id()).collect();
    assert_eq!(said, ["agent.select"]);

    let moved = crate::input::handle(
        crate::input::Input::Move { x: x + 40.0, y },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    let said: Vec<&str> = moved.iter().map(|one| one.id()).collect();
    assert_eq!(said, ["agent.select"], "and it carries while it is down");

    crate::input::handle(crate::input::Input::Release, &mut ui, &app, &hits, window());
    assert!(
        !matches!(ui.held, Some(crate::Held::Selecting(crate::Grid::Agent))),
        "the release ends it"
    );
}

#[test]
fn the_chord_copies_what_the_screen_holds() {
    let app = offering(Vec::new());
    let mut ui = session_ui();
    let chord = crate::tests::CTRL_SHIFT;
    let acted = crate::tests::press(crate::input::Key::Char('c'), chord, &mut ui, &app);
    let said: Vec<&str> = acted.iter().map(|one| one.id()).collect();
    assert_eq!(said, ["agent.copy"]);
}

#[test]
fn a_trackpad_that_moves_a_little_still_moves_the_screen() {
    let app = offering(Vec::new());
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let (x, y) = on_screen(&ui);
    let nudge = |ui: &mut Ui| {
        crate::input::handle(
            crate::input::Input::Scroll {
                x,
                y,
                delta: crate::input::Delta::Pixels {
                    across: 0.0,
                    down: 7.0,
                },
            },
            ui,
            &app,
            &hits,
            window(),
        )
    };
    let mut asked = 0;
    for _ in 0..3 {
        asked += nudge(&mut ui).len();
    }
    assert!(asked > 0, "what is under a line is carried, not lost");
}
