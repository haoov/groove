use groove_gfx::Fonts;
use groove_types::SessionId;

use crate::hit::Target;
use crate::input::Key;
use crate::tests::{ALT_SHIFT, click, full_app, press, window};
use crate::views::session::Tab;
use crate::{Focus, Ui, view};

fn on_diff(app: &groove_controllers::AppState) -> (Ui, crate::Hits) {
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    let (_, hits) = view(app, &ui, window(), &mut Fonts::embedded());
    (ui, hits)
}

#[test]
fn a_click_hands_the_keyboard_to_the_pane_it_lands_in() {
    let app = full_app();
    let (mut ui, hits) = on_diff(&app);
    assert_eq!(ui.focus, Focus::Agent, "the agent has it at the start");

    let rail = hits
        .rect_of(&Target::Session(SessionId::new("a")))
        .expect("a rail row");
    click(rail, &mut ui, &app, &hits);
    assert_eq!(ui.focus, Focus::Rail);

    let agent = hits.rect_of(&Target::Agent).expect("the agent pane");
    click(agent, &mut ui, &app, &hits);
    assert_eq!(ui.focus, Focus::Agent);
}

#[test]
fn the_chord_walks_the_panes_left_to_right() {
    let app = full_app();
    let mut ui = Ui {
        focus: Focus::Rail,
        ..Ui::default()
    };
    for expected in [
        Focus::Agent,
        Focus::Workspace,
        Focus::Sidebar,
        Focus::Sidebar,
    ] {
        press(Key::Right, ALT_SHIFT, &mut ui, &app);
        assert_eq!(ui.focus, expected, "right stops at the last pane");
    }
    for expected in [Focus::Workspace, Focus::Agent, Focus::Rail, Focus::Rail] {
        press(Key::Left, ALT_SHIFT, &mut ui, &app);
        assert_eq!(ui.focus, expected, "and left at the first");
    }
}

#[test]
fn a_key_reaches_the_agent_only_while_the_agent_has_the_keyboard() {
    let app = full_app();
    let mut ui = Ui::default();
    let typed = press(
        Key::Char('a'),
        crate::input::Modifiers::default(),
        &mut ui,
        &app,
    );
    assert_eq!(typed.len(), 1, "the agent hears it");
    assert_eq!(typed[0].id(), "agent.send");

    ui.focus = Focus::Workspace;
    let quiet = press(
        Key::Char('a'),
        crate::input::Modifiers::default(),
        &mut ui,
        &app,
    );
    assert!(quiet.is_empty(), "the workspace does not pass it on");
}

#[test]
fn the_rail_walks_its_sessions() {
    let app = full_app();
    let mut ui = Ui {
        focus: Focus::Rail,
        ..Ui::default()
    };
    let down = press(Key::Down, crate::input::Modifiers::default(), &mut ui, &app);
    assert_eq!(down.len(), 1);
    assert_eq!(down[0].id(), "session.select");
}

#[test]
fn a_click_on_the_agent_takes_the_keys_from_the_search_bars() {
    let app = full_app();
    let (mut ui, hits) = on_diff(&app);
    ui.focus = Focus::Workspace;
    let ctrl = crate::input::Modifiers {
        ctrl: true,
        ..Default::default()
    };
    press(Key::Char('f'), ctrl, &mut ui, &app);
    assert!(ui.session.find.as_ref().is_some_and(|find| find.typing));
    let agent = hits.rect_of(&Target::Agent).expect("the agent pane");
    click(agent, &mut ui, &app, &hits);
    assert!(ui.session.find.as_ref().is_some_and(|find| !find.typing));
    let escape = press(Key::Escape, Default::default(), &mut ui, &app);
    assert!(
        ui.session.find.is_some(),
        "Escape is the agent's, not the bar's"
    );
    assert!(!escape.is_empty(), "the agent hears it");
}

#[test]
fn a_click_on_the_overview_takes_the_keys_from_the_agent() {
    let app = full_app();
    let mut ui = Ui::default();
    assert_eq!(ui.session.tab, Tab::Overview);
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let ground = hits.rect_of(&Target::Overview).expect("the overview");
    let (x, y) = (ground.x + ground.w / 2.0, ground.bottom() - 2.0);
    crate::tests::pressed(x, y, &mut ui, &app, &hits);
    assert_eq!(ui.focus, Focus::Workspace);
    let typed = press(
        Key::Char('a'),
        crate::input::Modifiers::default(),
        &mut ui,
        &app,
    );
    assert!(typed.is_empty(), "the agent no longer hears the keys");
}

#[test]
fn the_focused_region_alone_carries_the_line() {
    let app = full_app();
    let tokens = groove_ui_kit::base::tokens::Tokens::new(1.0);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), tokens);
    let accent = styles.color(groove_ui_kit::base::style::Role::Accent);
    let thick = tokens.hairline * 2.0;
    for focus in [
        Focus::Rail,
        Focus::Agent,
        Focus::Workspace,
        Focus::Sidebar,
        Focus::Terminal,
    ] {
        let mut ui = Ui::default();
        ui.session.tab = Tab::Diff;
        ui.session.manual = true;
        ui.focus = focus;
        let layout = crate::layout::Layout::of(window(), &ui);
        let region = match focus {
            Focus::Rail => layout.rail,
            Focus::Agent => layout.agent_bar,
            Focus::Workspace => layout.workspace,
            Focus::Sidebar => layout.commit,
            Focus::Terminal => layout.manual,
        };
        let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
        let regions = [
            layout.rail,
            layout.agent_bar,
            layout.workspace,
            layout.commit,
            layout.manual,
        ];
        let under = |quad: &&groove_gfx::Quad| {
            let at = (quad.rect.x, quad.rect.w, quad.rect.bottom());
            regions.iter().any(|one| (one.x, one.w, one.bottom()) == at)
        };
        let lines: Vec<_> = frame
            .layers()
            .iter()
            .flat_map(|one| one.quads.iter())
            .filter(|quad| quad.color == accent && quad.rect.h == thick)
            .filter(under)
            .collect();
        assert_eq!(lines.len(), 1, "{focus:?}: one line, under no other region");
        assert_eq!(lines[0].rect.bottom(), region.bottom(), "{focus:?}");
        assert_eq!(
            (lines[0].rect.x, lines[0].rect.w),
            (region.x, region.w),
            "{focus:?}"
        );
    }
}

#[test]
fn alt_and_a_number_hand_the_keys_to_a_region() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    for (key, focus) in [
        ('1', Focus::Rail),
        ('2', Focus::Agent),
        ('3', Focus::Workspace),
        ('4', Focus::Sidebar),
    ] {
        press(Key::Char(key), crate::tests::ALT, &mut ui, &app);
        assert_eq!(ui.focus, focus, "alt+{key}");
    }
}

#[test]
fn a_second_press_folds_the_sidebar_away() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    press(Key::Char('4'), crate::tests::ALT, &mut ui, &app);
    assert!(!ui.session.folded && ui.focus == Focus::Sidebar);
    press(Key::Char('4'), crate::tests::ALT, &mut ui, &app);
    assert!(ui.session.folded, "folded on the second press");
    assert_eq!(
        ui.focus,
        Focus::Workspace,
        "the keys go back to the workspace"
    );
    press(Key::Char('4'), crate::tests::ALT, &mut ui, &app);
    assert!(!ui.session.folded && ui.focus == Focus::Sidebar, "and back");
}

#[test]
fn on_the_rail_enter_goes_to_the_workspace_and_a_and_v_answer_the_ask() {
    use crate::input::Modifiers;
    let app = crate::tests::bar::asking(vec![crate::tests::bar::ask("w1", "write", "src/lib.rs")]);
    let mut ui = Ui {
        focus: Focus::Rail,
        ..Default::default()
    };
    let approved = press(Key::Char('a'), Modifiers::default(), &mut ui, &app);
    assert!(matches!(
        &approved[..],
        [groove_controllers::Command::Agent(groove_controllers::agent::Command::Approve { id })]
            if id.as_str() == "w1"
    ));
    press(Key::Char('v'), Modifiers::default(), &mut ui, &app);
    assert!(
        matches!(ui.overlay, Some(crate::Overlay::Examining(_))),
        "the sheet shows it"
    );
    ui.overlay = None;
    press(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert_eq!(ui.focus, Focus::Workspace);
}

#[test]
fn alt_brackets_step_through_the_terminal_tabs() {
    let mut app = full_app();
    let session = SessionId::new("a");
    let first = app.shell.reserve(&session, false);
    let second = app.shell.reserve(&session, false);
    let tabs: Vec<u64> = app
        .shell
        .shells(&session)
        .expect("shells")
        .tabs
        .iter()
        .map(|tab| tab.id)
        .collect();
    assert_eq!(tabs.len(), 2, "{first} and {second} each in a tab");
    let mut ui = Ui::default();
    (ui.focus, ui.session.manual) = (Focus::Terminal, true);
    let selected = app.shell.shells(&session).and_then(|one| one.selected);
    let next = press(Key::Char(']'), crate::tests::ALT, &mut ui, &app);
    let other = tabs
        .iter()
        .copied()
        .find(|tab| Some(*tab) != selected)
        .expect("another tab");
    assert!(
        matches!(
            &next[..],
            [groove_controllers::Command::Shell(groove_controllers::shell::Command::SelectTab { tab, .. })]
                if *tab == other
        ),
        "{next:?}"
    );
    let previous = press(Key::Char('['), crate::tests::ALT, &mut ui, &app);
    assert!(
        matches!(&previous[..], [groove_controllers::Command::Shell(_)]),
        "{previous:?}"
    );
}

#[test]
fn alt_brackets_step_through_the_workspace_tabs_elsewhere() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::Overview;
    press(Key::Char(']'), crate::tests::ALT, &mut ui, &app);
    assert_eq!(ui.session.tab, Tab::Diff);
    press(Key::Char(']'), crate::tests::ALT, &mut ui, &app);
    assert_eq!(ui.session.tab, Tab::Files);
    press(Key::Char(']'), crate::tests::ALT, &mut ui, &app);
    assert_eq!(
        ui.session.tab,
        Tab::Overview,
        "no cluster held: Kubernetes is skipped"
    );
    press(Key::Char('['), crate::tests::ALT, &mut ui, &app);
    assert_eq!(ui.session.tab, Tab::Files);
}
