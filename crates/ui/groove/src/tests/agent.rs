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
    let terminal =
        Terminal::spawn(spec, groove_types::AnsiPalette::MOCHA, hooks).expect("a terminal");
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

#[test]
fn dragging_over_the_agent_screen_holds_the_cells_it_crossed() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services) = working(home.path(), &spawner);
    printing(&mut state, "hello world");
    let mut ui = Ui {
        surface: groove_ui::Surface::Session,
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
