//! The pointer over the agent's screen, from the press down to what it holds.

use groove_controllers::agent_service::{Hooks, PtySpec, Terminal};
use groove_controllers::{AppState, Services, SyncSpawner, dispatch};
use groove_ui::input::{Input, handle};
use groove_ui::{Ui, view};

use super::explorer::{window, working};

fn printing(state: &mut AppState, said: &str) {
    let spec = PtySpec {
        program: "sh".into(),
        args: vec!["-c".into(), format!("printf '{said}'; sleep 30")],
        cwd: "/".into(),
        env: Vec::new(),
        rows: 24,
        cols: 80,
    };
    let hooks = Hooks {
        on_damage: Box::new(|| {}),
        on_exit: Box::new(|_| {}),
    };
    let runtime = groove_controllers::SyncSpawner::new().expect("a runtime");
    let terminal = runtime
        .block_on(Terminal::spawn(
            spec,
            groove_types::AnsiPalette::MOCHA,
            hooks,
        ))
        .expect("a terminal");
    let shown = || terminal.screen().cells.iter().any(|one| one.ch == 'h');
    for _ in 0..1000 {
        if shown() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(shown(), "the program printed what it was asked to");
    let session = state.session.selected.clone().expect("a session");
    state.agent.agents.retain(|(id, _)| id != &session);
    state.agent.agents.push((
        session,
        groove_controllers::agent_service::Agent {
            terminal: Some(terminal),
            activity: groove_types::SessionActivity {
                status: groove_types::AgentStatus::Idle,
                tool: None,
                asks: Vec::new(),
                changed_at: groove_types::Timestamp::now(),
                seen_at: None,
            },
            started_at: groove_types::Timestamp::now(),
            launch: 0,
        },
    ));
}

/// The agent pane showing the terminal in place of the conversation.
fn terminal() -> groove_ui::AgentUi {
    groove_ui::AgentUi {
        terminal: true,
        ..Default::default()
    }
}

#[test]
fn dragging_over_the_agent_screen_holds_the_cells_it_crossed() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services) = working(home.path(), &spawner);
    printing(&mut state, "hello world");
    let mut ui = Ui {
        surface: groove_ui::Surface::Session,
        agent: terminal(),
        ..Ui::default()
    };

    let (_, hits) = view(&state, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let pane = hits
        .rect_of(&groove_ui::Target::Agent)
        .expect("the agent pane is on screen");
    let cell = window().cell;
    let (x, y) = (
        pane.x + 8.0 + cell.width / 2.0,
        pane.y + 8.0 + cell.height / 2.0,
    );
    let acted = handle(
        Input::Press {
            x,
            y,
            mods: Default::default(),
        },
        &mut ui,
        &state,
        &hits,
        window(),
    );
    assert!(ui.pointing(), "the press claims the moves that follow");
    run(acted, &mut state, &services, &spawner);

    let acted = handle(
        Input::Move {
            x: x + 4.0 * cell.width,
            y,
        },
        &mut ui,
        &state,
        &hits,
        window(),
    );
    run(acted, &mut state, &services, &spawner);

    let held = state.agent.agents[0]
        .1
        .terminal
        .as_ref()
        .and_then(|one| one.selected());
    assert_eq!(held.as_deref(), Some("hello"));
}

fn run(
    commands: Vec<groove_controllers::Command>,
    state: &mut AppState,
    services: &Services,
    spawner: &SyncSpawner,
) {
    for command in commands {
        dispatch(command, state, services, spawner);
    }
}

#[test]
fn a_paste_with_no_text_on_the_clipboard_reaches_the_agent_as_the_key() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services) = working(home.path(), &spawner);
    let mut ui = Ui {
        surface: groove_ui::Surface::Session,
        agent: terminal(),
        focus: groove_ui::Focus::Agent,
        ..Ui::default()
    };
    let mods = groove_ui::input::Modifiers {
        ctrl: true,
        ..Default::default()
    };
    let mut unpasted = Some(crate::app::events::Unpasted {
        key: Some(Input::Key {
            key: groove_ui::input::Key::Char('v'),
            mods,
        }),
        aim: groove_ui::input::aim(&ui, &state),
    });
    let read = groove_controllers::workspace::Command::ReadClipboard;
    run(
        vec![groove_controllers::Command::Workspace(read)],
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);

    let aim = groove_ui::input::aim(&ui, &state);
    let input =
        crate::app::events::landed(&mut state, &mut unpasted, aim).expect("the key comes back");
    let (_, hits) = view(&state, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let asked = handle(input, &mut ui, &state, &hits, window());
    let session = state.session.selected.clone().expect("a session");
    let ctrl_v = groove_controllers::agent::Command::Send {
        session,
        bytes: vec![0x16],
    };
    assert_eq!(asked, vec![groove_controllers::Command::Agent(ctrl_v)]);
}

#[test]
fn a_paste_that_lands_after_the_keys_moved_is_dropped() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, _services) = working(home.path(), &spawner);
    let mut ui = Ui {
        surface: groove_ui::Surface::Session,
        agent: terminal(),
        focus: groove_ui::Focus::Agent,
        overlay: Some(groove_ui::Overlay::Palette(Default::default())),
        ..Ui::default()
    };
    let asked = |ui: &Ui, state: &AppState| {
        Some(crate::app::events::Unpasted {
            key: None,
            aim: groove_ui::input::aim(ui, state),
        })
    };
    let mut unpasted = asked(&ui, &state);
    ui.overlay = None;
    state.pasted = Some(Some("git@host:g/r.git".into()));
    let aim = groove_ui::input::aim(&ui, &state);
    let landed = crate::app::events::landed(&mut state, &mut unpasted, aim.clone());
    assert_eq!(landed, None, "the agent never gets the palette's text");

    let mut unpasted = asked(&ui, &state);
    state.pasted = Some(Some("one".into()));
    let landed = crate::app::events::landed(&mut state, &mut unpasted, aim);
    assert_eq!(landed, Some(Input::Paste("one".into())));
}

#[test]
fn a_paste_that_lands_after_another_session_is_selected_is_dropped() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, _services) = working(home.path(), &spawner);
    let ui = Ui {
        surface: groove_ui::Surface::Session,
        agent: terminal(),
        focus: groove_ui::Focus::Agent,
        ..Ui::default()
    };
    let ctrl_v = Input::Key {
        key: groove_ui::input::Key::Char('v'),
        mods: groove_ui::input::Modifiers {
            ctrl: true,
            ..Default::default()
        },
    };
    let was = state.session.selected.clone();
    for pasted in [Some("one".to_string()), None] {
        state.session.selected = was.clone();
        let mut unpasted = Some(crate::app::events::Unpasted {
            key: Some(ctrl_v.clone()),
            aim: groove_ui::input::aim(&ui, &state),
        });
        state.session.selected = Some(groove_types::SessionId::new("another"));
        state.pasted = Some(pasted.clone());
        let aim = groove_ui::input::aim(&ui, &state);
        let landed = crate::app::events::landed(&mut state, &mut unpasted, aim);
        assert_eq!(landed, None, "{pasted:?} never reaches the other agent");
    }
}
