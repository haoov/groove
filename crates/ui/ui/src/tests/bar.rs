//! The agent's own row: the write it waits on, or the skills it can be sent.

use groove_controllers::agent_service::Agent;
use groove_gfx::Fonts;
use groove_types::{AgentStatus, ApprovalId, Ask, SessionActivity, SessionId, Timestamp};

use crate::hit::Target;
use crate::tests::{app, click, window};
use crate::{Surface, Ui};

/// The fixture's session, with the writes it waits on.
pub(super) fn asking(asks: Vec<Ask>) -> groove_controllers::AppState {
    let mut app = app();
    let activity = SessionActivity {
        status: AgentStatus::Idle,
        tool: None,
        asks,
        auto_approve: false,
        changed_at: Timestamp::new(0),
        seen_at: None,
    };
    app.agent.agents.push((
        SessionId::new("a"),
        Agent {
            terminal: None,
            activity,
            started_at: Timestamp::new(0),
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
    let (frame, _) = crate::view(app, ui, window(), &mut Fonts::embedded());
    frame.layers()[0]
        .texts
        .iter()
        .map(|run| run.text.clone())
        .collect()
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
    app.agent.skills = skills;
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
        editable: false,
        enabled: true,
        changed_at: Timestamp::new(0),
    }
}

#[test]
fn the_bar_counts_the_skills_and_offers_them() {
    let app = offering(vec![skill("co-review", "co review", &[])]);
    let ui = session_ui();
    let drawn = drawn(&app, &ui);
    assert!(drawn.iter().any(|one| one == "skills"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "reload"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "1 skill"), "{drawn:?}");
}

#[test]
fn a_skill_switched_off_is_neither_counted_nor_offered() {
    let off = groove_types::Skill {
        enabled: false,
        ..skill("rollout", "rollout", &[])
    };
    let app = offering(vec![skill("co-review", "co review", &[]), off]);
    let drawn = drawn(&app, &session_ui());
    assert!(drawn.iter().any(|one| one == "1 skill"), "{drawn:?}");
}

#[test]
fn the_skills_menu_holds_what_this_kind_is_offered() {
    let app = offering(vec![
        skill("save-task", "save task", &["task"]),
        skill("convert-explorer", "convert to task", &["explorer"]),
    ]);
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let word = hits
        .rect_of(&Target::Skills(SessionId::new("a")))
        .expect("the skills word");
    assert!(
        click(word, &mut ui, &app, &hits).is_empty(),
        "it only opens"
    );

    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&crate::hit::Target::MenuRow(0))
        .expect("a row of it");
    let acted = click(row, &mut ui, &app, &hits);
    assert_eq!(
        acted,
        vec![groove_controllers::Command::Agent(
            groove_controllers::agent::Command::SendSkill {
                session: SessionId::new("a"),
                id: "groove:convert-explorer".into(),
                args: None,
            }
        )],
        "an explorer is offered the explorer's own"
    );
}

#[test]
fn reload_ends_the_agent_and_starts_it_again() {
    let app = offering(Vec::new());
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let word = hits
        .rect_of(&Target::Reload(SessionId::new("a")))
        .expect("the reload word");
    let acted = click(word, &mut ui, &app, &hits);
    let said: Vec<&str> = acted.iter().map(|one| one.id()).collect();
    assert_eq!(said, ["agent.reload"]);
}

#[test]
fn a_skill_newer_than_the_agent_lights_the_reload() {
    let reload = |app: &groove_controllers::AppState| {
        let (frame, _) = crate::view(app, &session_ui(), window(), &mut Fonts::embedded());
        let mut texts = frame.layers()[0].texts.iter();
        texts.find(|one| one.text == "reload").unwrap().style.color
    };
    let mut app = offering(vec![skill("save-task", "save task", &[])]);
    let theme = app.config.theme();
    let styles = groove_ui_kit::base::style::Styles::new(theme, crate::Tokens::new(1.0));
    let attention = styles.color(groove_ui_kit::base::style::Role::Attention);
    assert_ne!(reload(&app), attention, "no skill is newer");
    app.agent.skills[0].changed_at = Timestamp::new(50);
    assert_eq!(reload(&app), attention);
}

#[test]
fn the_row_stands_under_the_screen_and_the_grid_gives_way() {
    let app = offering(Vec::new());
    let ui = session_ui();
    let layout = crate::layout::Layout::of(window(), &ui);
    let tokens = crate::Tokens::new(1.0);
    assert_eq!(
        layout.agent_bar.bottom(),
        layout.agent.bottom(),
        "it sits at the pane's foot"
    );
    assert_eq!(layout.agent_bar.h, tokens.bar);
    assert_eq!(
        layout.agent_origin(&tokens).1,
        layout.agent.y + tokens.sm,
        "the screen starts at the top as it did"
    );

    let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let word = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text == "skills")
        .expect("the skills word");
    assert!(word.y >= layout.agent_bar.y, "and its words with it");
}

#[test]
fn the_skills_menu_stands_above_the_word_that_opens_it() {
    let app = offering(vec![skill("co-review", "co review", &[])]);
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let word = hits
        .rect_of(&Target::Skills(SessionId::new("a")))
        .expect("the skills word");
    click(word, &mut ui, &app, &hits);

    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&crate::hit::Target::MenuRow(0))
        .expect("a row of it");
    assert!(
        row.bottom() <= word.y + 1.0,
        "the menu is over the word, not on it: {} against {}",
        row.bottom(),
        word.y
    );
}

#[test]
fn the_skills_word_carries_the_caret_the_pickers_carry() {
    let app = offering(vec![skill("co-review", "co review", &[])]);
    let ui = session_ui();
    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let word = hits
        .rect_of(&Target::Skills(SessionId::new("a")))
        .expect("the skills word");
    let held = frame.layers()[0]
        .icons
        .iter()
        .filter(|one| one.icon == groove_gfx::Icon::CaretDown)
        .filter(|one| one.rect.x >= word.x && one.rect.right() <= word.right())
        .count();
    assert_eq!(held, 1, "one caret, inside the word's own box");
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

/// Every text of the frame, the menu's own layer included.
fn everywhere(frame: &groove_gfx::Frame) -> Vec<String> {
    frame
        .layers()
        .iter()
        .flat_map(|one| one.texts.iter())
        .map(|run| run.text.clone())
        .collect()
}

/// The config of a machine whose task sources are set up.
pub(super) fn sourced(
    mut app: groove_controllers::AppState,
    github: bool,
    notion: bool,
) -> groove_controllers::AppState {
    app.config.config = Some(groove_types::Config {
        notion: notion.then(|| groove_types::NotionConfig {
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
        }),
        github: github.then(|| groove_types::GithubConfig {
            host: "github.com".into(),
            token: None,
            properties: Default::default(),
            status_map: Default::default(),
            priority_map: Default::default(),
        }),
        git: groove_types::GitConfig {
            worktree_root: "code/worktrees".into(),
        },
        ui: Default::default(),
        preferences: Default::default(),
        keymap: Default::default(),
        shared: None,
        skills_off: Vec::new(),
        routines: Default::default(),
    });
    app
}

#[test]
fn a_conversion_files_in_each_source_that_is_set_up() {
    let app = sourced(
        offering(vec![skill("convert-explorer", "convert to task", &[])]),
        true,
        true,
    );
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let word = hits
        .rect_of(&Target::Skills(SessionId::new("a")))
        .expect("the skills word");
    click(word, &mut ui, &app, &hits);

    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let drawn = everywhere(&frame);
    assert!(
        drawn.iter().any(|one| one == "convert to task in GitHub"),
        "{drawn:?}"
    );
    assert!(
        drawn.iter().any(|one| one == "convert to task in Notion"),
        "{drawn:?}"
    );
    assert!(
        !drawn.iter().any(|one| one == "convert to task"),
        "{drawn:?}"
    );

    let row = hits
        .rect_of(&crate::hit::Target::MenuRow(0))
        .expect("a row of it");
    let acted = click(row, &mut ui, &app, &hits);
    assert_eq!(
        acted,
        vec![groove_controllers::Command::Agent(
            groove_controllers::agent::Command::SendSkill {
                session: SessionId::new("a"),
                id: "groove:convert-explorer".into(),
                args: Some("github".into()),
            }
        )]
    );
}

#[test]
fn filing_a_task_stays_off_the_menu_and_converting_names_each_source() {
    let app = sourced(
        offering(vec![
            skill("create-task", "create task", &[]),
            skill("convert-explorer", "convert to task", &[]),
        ]),
        true,
        true,
    );
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let word = hits
        .rect_of(&Target::Skills(SessionId::new("a")))
        .expect("the skills word");
    click(word, &mut ui, &app, &hits);

    let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let drawn = everywhere(&frame);
    let rows: Vec<&String> = drawn.iter().filter(|one| one.contains(" in ")).collect();
    let mut once = rows.clone();
    once.sort();
    once.dedup();
    assert_eq!(
        rows.len(),
        2,
        "two sources, one skill on the menu: {rows:?}"
    );
    assert!(
        !drawn.iter().any(|one| one.starts_with("create task")),
        "{drawn:?}"
    );
    assert_eq!(
        once.len(),
        rows.len(),
        "no row says what another says: {rows:?}"
    );
}

#[test]
fn one_source_files_without_naming_it() {
    let app = sourced(
        offering(vec![skill("convert-explorer", "convert to task", &[])]),
        false,
        true,
    );
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let word = hits
        .rect_of(&Target::Skills(SessionId::new("a")))
        .expect("the skills word");
    click(word, &mut ui, &app, &hits);
    let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let drawn = everywhere(&frame);
    assert!(
        drawn.iter().any(|one| one == "convert to task"),
        "{drawn:?}"
    );
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
        matches!(ui.held, Some(crate::Held::AgentText)),
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
        !matches!(ui.held, Some(crate::Held::AgentText)),
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
