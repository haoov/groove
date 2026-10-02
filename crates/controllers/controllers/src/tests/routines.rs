//! Routines switched on with their scope, off, and one trigger at a time; the file written each time.

use groove_agent_service::routines::{Listed, parse};
use groove_types::Trigger;

use crate::config::Command;
use crate::tests::fixture::fresh;
use crate::{AppState, Command as Cmd, Services, SyncSpawner, dispatch};

const FIX_CI: &str = "---\nskills: groove:fix-ci\nkind: bound\non: ci-failed, changes-requested\n\
                      scope: edit, commit, push\n---\n";

fn listed(state: &mut AppState) {
    let id = "user:fix-red-ci";
    state.agent.routines = vec![
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

fn send(command: Command, state: &mut AppState, services: &Services, spawner: &SyncSpawner) {
    dispatch(Cmd::Config(command), state, services, spawner);
    spawner.drain(state, services);
}

fn switch(id: &str, on: bool) -> Command {
    Command::SwitchRoutine { id: id.into(), on }
}

#[test]
fn a_routine_switched_on_is_written_and_one_of_its_triggers_turned_off_alone() {
    let (home, spawner, services, mut state) = fresh();
    listed(&mut state);
    send(
        switch("user:fix-red-ci", true),
        &mut state,
        &services,
        &spawner,
    );
    let quiet = Command::SwitchTrigger {
        id: "user:fix-red-ci".into(),
        trigger: Trigger::ChangesRequested,
        on: false,
    };
    send(quiet, &mut state, &services, &spawner);
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
        switch("user:fix-red-ci", false),
        &mut state,
        &services,
        &spawner,
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
    send(switch("user:broken", true), &mut state, &services, &spawner);
    send(switch("user:gone", true), &mut state, &services, &spawner);
    assert_eq!(state.errors.len(), 2);
    assert!(state.config.routines().on.is_empty());
}

const DIGEST: &str =
    "---\nskills: platform:follow-ups\nkind: standalone\non: daily\n---\nList them.\n";

#[test]
fn a_standalone_routine_switched_on_has_its_session_on_the_rail_and_off_loses_it() {
    let (_home, spawner, services, mut state) = fresh();
    let id = "user:digest";
    state.agent.routines = vec![Listed {
        id: id.into(),
        read: parse(id, "digest", DIGEST),
    }];
    send(switch(id, true), &mut state, &services, &spawner);
    send(switch(id, true), &mut state, &services, &spawner);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    let routines: Vec<_> = state
        .session
        .open
        .iter()
        .filter(|one| one.session.kind.routine() == Some(id))
        .collect();
    assert_eq!(routines.len(), 1, "switched on twice, one session");
    let opened = spawner.block_on(services.session.opened()).unwrap();
    assert!(opened.iter().any(|(one, _)| one.kind.routine() == Some(id)));

    send(switch(id, false), &mut state, &services, &spawner);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(state.session.open.is_empty());
    let living = spawner.block_on(services.session.living()).unwrap();
    assert!(living.is_empty(), "{living:?}");
}
