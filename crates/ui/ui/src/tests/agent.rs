//! The pointer on the agent's pane: whose selection it is, and where a press goes.

use groove_gfx::Fonts;
use groove_types::Ask;

use crate::hit::Target;
use crate::tests::window;
use crate::{Surface, Ui};

fn agent_app(asks: Vec<Ask>) -> groove_controllers::AppState {
    super::bar::asking(asks)
}

fn session_ui() -> Ui {
    Ui {
        surface: Surface::Session,
        ..Ui::default()
    }
}

fn reading_mouse(app: &mut groove_controllers::AppState) {
    use groove_controllers::agent_service::{Hooks, PtySpec, Terminal};
    let spec = PtySpec {
        program: "sh".into(),
        args: vec![
            "-c".into(),
            "printf '\\033[?1000h\\033[?1006h'; sleep 30".into(),
        ],
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
    for _ in 0..100 {
        if terminal.reads_mouse() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(terminal.reads_mouse(), "the program asked for the mouse");
    app.agent.agents[0].1.terminal = Some(terminal);
}

#[test]
fn a_program_that_reads_the_mouse_keeps_the_pointer_and_shift_takes_it_back() {
    let mut app = agent_app(Vec::new());
    reading_mouse(&mut app);
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let pane = hits.rect_of(&Target::Agent).expect("the agent pane");
    let press = |ui: &mut Ui, shift: bool| {
        crate::input::handle(
            crate::input::Input::Press {
                x: pane.x + pane.w / 2.0,
                y: pane.y + pane.h / 2.0,
                mods: crate::input::Modifiers {
                    shift,
                    ..Default::default()
                },
            },
            ui,
            &app,
            &hits,
            window(),
        )
    };

    let acted = press(&mut ui, false);
    let said: Vec<&str> = acted.iter().map(|one| one.id()).collect();
    assert_eq!(said, ["agent.click"], "the program's own selection stands");
    assert!(!ui.agent.selecting, "and we take none of our own");

    let released =
        crate::input::handle(crate::input::Input::Release, &mut ui, &app, &hits, window());
    let said: Vec<&str> = released.iter().map(|one| one.id()).collect();
    assert_eq!(said, ["agent.click"], "the button going up is sent on too");

    let acted = press(&mut ui, true);
    let said: Vec<&str> = acted.iter().map(|one| one.id()).collect();
    assert_eq!(said, ["agent.select"], "shift takes the pointer back");
    assert!(ui.agent.selecting);
}

#[test]
fn a_press_that_begins_a_selection_claims_the_moves_that_follow() {
    let app = agent_app(Vec::new());
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let pane = hits.rect_of(&Target::Agent).expect("the agent pane");
    let (x, y) = (pane.x + pane.w / 2.0, pane.y + pane.h / 2.0);

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
    assert_eq!(acted.len(), 1, "the selection begins");
    assert!(ui.pointing(), "so the window keeps sending the moves");

    let acted = crate::input::handle(
        crate::input::Input::Move { x: x + 40.0, y },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    let said: Vec<&str> = acted.iter().map(|one| one.id()).collect();
    assert_eq!(said, ["agent.select"], "and each one carries it further");

    let acted = crate::input::handle(crate::input::Input::Release, &mut ui, &app, &hits, window());
    let said: Vec<&str> = acted.iter().map(|one| one.id()).collect();
    assert_eq!(said, ["agent.copy"], "letting go takes what it holds");
    assert!(!ui.pointing());
}
