//! Settings › Agent's routines: each one switched on only once its scope is allowed, then its triggers.

use groove_controllers::agent_service::routines::{Listed, parse};
use groove_controllers::{AppState, Command, config};
use groove_gfx::Fonts;
use groove_types::Trigger;

use super::*;
use crate::hit::Target;
use crate::view;

const FIX_CI: &str = "---\ndescription: fix it\nskills: groove:fix-ci\nkind: bound\n\
                      on: ci-failed\nscope: edit, commit, push\n---\n";

/// Settings › Agent over one routine that reads and one that does not.
fn routed(on: bool) -> (AppState, Ui) {
    let mut app = crate::tests::bar::sourced(full_app(), false, false);
    let id = "user:fix-red-ci";
    app.agent.routines = vec![
        Listed {
            id: id.into(),
            read: parse(id, "fix-red-ci", FIX_CI),
        },
        Listed {
            id: "user:broken".into(),
            read: Err("it names no `skill`".into()),
        },
    ];
    if on && let Some(config) = app.config.config.as_mut() {
        config.routines.on = vec![id.into()];
    }
    let mut ui = Ui::default();
    ui.settings.open = true;
    ui.settings.section = crate::views::settings::Section::Agent;
    (app, ui)
}

fn drawn(app: &AppState, ui: &Ui) -> (Vec<String>, Hits) {
    let tall = metrics(1280, 2400, 1.0);
    let (frame, hits) = view(app, ui, tall, &mut Fonts::embedded());
    let texts = frame.layers()[0]
        .texts
        .iter()
        .map(|one| one.text.clone())
        .collect();
    (texts, hits)
}

fn clicked(app: &AppState, ui: &mut Ui, target: Target) -> Vec<Command> {
    let (_, hits) = drawn(app, ui);
    let rect = hits
        .rect_of(&target)
        .unwrap_or_else(|| panic!("{target:?} is drawn"));
    let input = crate::input::Input::Press {
        x: rect.x + rect.w / 2.0,
        y: rect.y + rect.h / 2.0,
        mods: Default::default(),
    };
    crate::input::handle(input, ui, app, &hits, metrics(1280, 2400, 1.0))
}

#[test]
fn a_routine_is_switched_on_only_once_its_scope_is_allowed() {
    let (app, mut ui) = routed(false);
    let (texts, _) = drawn(&app, &ui);
    for shown in [
        "Routines",
        "pause all",
        "at once",
        "user:fix-red-ci",
        "it names no `skill`",
    ] {
        assert!(texts.iter().any(|t| t == shown), "{shown}: {texts:?}");
    }
    let asked = clicked(&app, &mut ui, Target::RoutineOn("user:fix-red-ci".into()));
    assert!(asked.is_empty(), "the first click asks");
    let (texts, _) = drawn(&app, &ui);
    let question = "allow without asking: edit, commit, push?";
    assert!(texts.iter().any(|t| t == question), "{texts:?}");
    let asked = clicked(
        &app,
        &mut ui,
        Target::RoutineAllow("user:fix-red-ci".into()),
    );
    let on = config::Command::SwitchRoutine {
        id: "user:fix-red-ci".into(),
        on: true,
    };
    assert_eq!(asked, [Command::Config(on)]);
}

#[test]
fn a_routine_switched_on_shows_its_triggers_each_with_its_switch() {
    let (app, mut ui) = routed(true);
    let (texts, hits) = drawn(&app, &ui);
    assert!(texts.iter().any(|t| t == "  ci-failed"), "{texts:?}");
    let quiet = Target::TriggerSwitch("user:fix-red-ci".into(), Trigger::CiFailed, false);
    assert!(hits.rect_of(&quiet).is_some());
    let asked = clicked(&app, &mut ui, quiet);
    let switched = config::Command::SwitchTrigger {
        id: "user:fix-red-ci".into(),
        trigger: Trigger::CiFailed,
        on: false,
    };
    assert_eq!(asked, [Command::Config(switched)]);
}
