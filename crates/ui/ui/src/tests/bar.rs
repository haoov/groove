//! The agent's own row: the write it waits on, or the skills it can be sent.

use groove_controllers::agent_service::Agent;
use groove_gfx::Fonts;
use groove_types::{AgentStatus, ApprovalId, Ask, SessionActivity, SessionId, Timestamp};

use crate::hit::Target;
use crate::tests::{app, click, window};
use crate::{Surface, Ui};

/// The fixture's session, with the writes it waits on.
fn asking(asks: Vec<Ask>) -> groove_controllers::AppState {
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

fn ask(id: &str, op: &str, subject: &str) -> Ask {
    Ask {
        id: ApprovalId::new(id),
        op: op.to_string(),
        subject: subject.to_string(),
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

fn session_ui() -> Ui {
    Ui {
        surface: Surface::Session,
        ..Ui::default()
    }
}

#[test]
fn the_band_says_the_write_and_offers_the_two_answers() {
    let app = asking(vec![ask("ap-1", "git_commit", "fix: one")]);
    let drawn = drawn(&app, &session_ui());
    assert!(drawn.iter().any(|one| one == "git_commit"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "fix: one"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "approve"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "refuse"), "{drawn:?}");
}

#[test]
fn nothing_stands_there_when_no_write_waits() {
    let app = asking(Vec::new());
    let drawn = drawn(&app, &session_ui());
    assert!(!drawn.iter().any(|one| one == "approve"), "{drawn:?}");
}

#[test]
fn the_band_says_how_many_writes_stand_behind_the_first() {
    let app = asking(vec![
        ask("ap-1", "git_commit", "fix: one"),
        ask("ap-2", "git_push", ""),
    ]);
    let drawn = drawn(&app, &session_ui());
    assert!(drawn.iter().any(|one| one.contains("+1 more")), "{drawn:?}");
    assert!(!drawn.iter().any(|one| one == "git_push"), "one at a time");
}

#[test]
fn approving_asks_the_controller_to_run_it() {
    let app = asking(vec![ask("ap-1", "git_commit", "fix: one")]);
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let target = Target::Approve(ApprovalId::new("ap-1"));
    let box_ = hits.rect_of(&target).expect("the approve button");
    let acted = click(box_, &mut ui, &app, &hits);
    assert_eq!(
        acted,
        vec![groove_controllers::Command::Agent(
            groove_controllers::agent::Command::Approve {
                id: ApprovalId::new("ap-1")
            }
        )]
    );
}

#[test]
fn refusing_asks_the_controller_to_drop_it() {
    let app = asking(vec![ask("ap-1", "git_commit", "fix: one")]);
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let target = Target::Refuse(ApprovalId::new("ap-1"));
    let box_ = hits.rect_of(&target).expect("the refuse button");
    let acted = click(box_, &mut ui, &app, &hits);
    assert_eq!(
        acted,
        vec![groove_controllers::Command::Agent(
            groove_controllers::agent::Command::Refuse {
                id: ApprovalId::new("ap-1")
            }
        )]
    );
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
fn the_skills_menu_holds_what_this_kind_is_offered() {
    let app = offering(vec![
        skill("save-task", "save task", &["task"]),
        skill("create-task", "create task", &["explorer"]),
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
                id: "groove:create-task".into(),
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
fn a_skill_newer_than_the_agent_says_so_on_the_row() {
    let mut app = offering(vec![skill("save-task", "save task", &[])]);
    app.agent.skills[0].changed_at = Timestamp::new(50);
    let drawn = drawn(&app, &session_ui());
    assert!(
        drawn.iter().any(|one| one.contains("newer than the agent")),
        "{drawn:?}"
    );
}

#[test]
fn the_write_it_waits_on_takes_the_row_from_the_skills() {
    let app = asking(vec![ask("ap-1", "git_commit", "fix: one")]);
    let drawn = drawn(&app, &session_ui());
    assert!(drawn.iter().any(|one| one == "git_commit"), "{drawn:?}");
    assert!(!drawn.iter().any(|one| one == "skills"), "{drawn:?}");
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
fn sourced(
    mut app: groove_controllers::AppState,
    github: bool,
    notion: bool,
) -> groove_controllers::AppState {
    app.config.config = Some(groove_types::Config {
        notion: notion.then(|| groove_types::NotionConfig {
            token: "t".into(),
            database_id: "d".into(),
            user_id: "u".into(),
            assignee: None,
            sprint: None,
            sprint_status: None,
            properties: Default::default(),
            status_map: Default::default(),
            priority_map: Default::default(),
            filters: groove_types::FilterConfig {
                exclude_statuses: Vec::new(),
                filter_by_assignee: true,
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
    });
    app
}

#[test]
fn a_task_files_in_each_source_that_is_set_up() {
    let app = sourced(
        offering(vec![skill("create-task", "create task", &[])]),
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
    assert!(drawn.iter().any(|one| one == "file in GitHub"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "file in Notion"), "{drawn:?}");
    assert!(!drawn.iter().any(|one| one == "create task"), "{drawn:?}");

    let row = hits
        .rect_of(&crate::hit::Target::MenuRow(0))
        .expect("a row of it");
    let acted = click(row, &mut ui, &app, &hits);
    assert_eq!(
        acted,
        vec![groove_controllers::Command::Agent(
            groove_controllers::agent::Command::SendSkill {
                session: SessionId::new("a"),
                id: "groove:create-task".into(),
                args: Some("github".into()),
            }
        )]
    );
}

#[test]
fn one_source_files_without_naming_it() {
    let app = sourced(
        offering(vec![skill("create-task", "create task", &[])]),
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
    assert!(drawn.iter().any(|one| one == "create task"), "{drawn:?}");
}
