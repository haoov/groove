//! Routines switched on, off, and one trigger at a time; the file written each time.

use groove_routine_service::routines::{Listed, parse};
use groove_types::Trigger;

use crate::AppState;
use crate::config::Command;
use crate::tests::fixture::{fresh, send};

const FIX_CI: &str =
    "---\nskills: groove:fix-ci\nkind: bound\non: ci-failed, changes-requested\n---\n";

fn listed(state: &mut AppState) {
    let id = "user:fix-red-ci";
    state.routine.routines = vec![
        Listed {
            id: id.into(),
            read: parse(id, "fix-red-ci", FIX_CI),
        },
        Listed {
            id: "user:broken".into(),
            read: Err("it names no `skill`".into()),
        },
    ];
}

fn switch(id: &str, on: bool) -> Command {
    Command::SwitchRoutine { id: id.into(), on }
}

#[test]
fn a_routine_switched_on_is_written_and_one_of_its_triggers_turned_off_alone() {
    let (home, spawner, services, mut state) = fresh();
    listed(&mut state);
    send(
        &mut state,
        &services,
        &spawner,
        switch("user:fix-red-ci", true),
    );
    let quiet = Command::SwitchTrigger {
        id: "user:fix-red-ci".into(),
        trigger: Trigger::ChangesRequested,
        on: false,
    };
    send(&mut state, &services, &spawner, quiet);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    let written = groove_config_service::load(&home.path().join("config"))
        .unwrap()
        .unwrap();
    assert_eq!(written.routines.on, ["user:fix-red-ci"]);
    assert_eq!(
        written.routines.quiet["user:fix-red-ci"],
        [Trigger::ChangesRequested]
    );

    send(
        &mut state,
        &services,
        &spawner,
        switch("user:fix-red-ci", false),
    );
    let held = state.config.routines();
    assert!(held.on.is_empty());
    assert!(
        held.quiet.is_empty(),
        "switched off, its turned-off triggers are forgotten"
    );
}

#[test]
fn a_routine_whose_file_does_not_read_is_not_switched_on() {
    let (_home, spawner, services, mut state) = fresh();
    listed(&mut state);
    send(&mut state, &services, &spawner, switch("user:broken", true));
    send(&mut state, &services, &spawner, switch("user:gone", true));
    assert_eq!(state.errors.len(), 2);
    assert!(state.config.routines().on.is_empty());
}
