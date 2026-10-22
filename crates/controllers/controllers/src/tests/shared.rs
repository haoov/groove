//! The team's shared repo: named once it reads as a marketplace, refused when not, let go.

use std::path::Path;

use groove_types::SharedConfig;

use crate::config::Command;
use crate::tests::fixture::{fresh, send, sh, until};
use crate::{AppState, Command as Cmd, Services, SyncSpawner, dispatch};

/// A bare origin whose `main` holds a marketplace of one plugin named `name`; its URL.
fn origin(home: &Path, name: &str) -> String {
    let (bare, work) = (home.join("team.git"), home.join("team"));
    let plugin = work.join("plugins").join(name).join(".claude-plugin");
    std::fs::create_dir_all(&bare).unwrap();
    std::fs::create_dir_all(work.join(".claude-plugin")).unwrap();
    std::fs::create_dir_all(&plugin).unwrap();
    sh(&bare, &["init", "--bare", "--initial-branch=main", "."]);
    sh(&work, &["init", "--initial-branch=main", "."]);
    let source = format!("./plugins/{name}");
    let listed = serde_json::json!({
        "name": "groove-agent", "owner": {}, "plugins": [{ "name": name, "source": source }]
    });
    std::fs::write(
        work.join(".claude-plugin").join("marketplace.json"),
        listed.to_string(),
    )
    .unwrap();
    let skill = work
        .join("plugins")
        .join(name)
        .join("skills")
        .join("rollout");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(
        skill.join("SKILL.md"),
        "---\ndescription: roll out\n---\nRoll out.\n",
    )
    .unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        format!(r#"{{ "name": "{name}" }}"#),
    )
    .unwrap();
    sh(&work, &["add", "."]);
    sh(&work, &["commit", "-m", "the plugin"]);
    sh(&work, &["remote", "add", "origin", bare.to_str().unwrap()]);
    sh(&work, &["push", "origin", "main"]);
    format!("file://{}", bare.display())
}

fn join(url: &str) -> Command {
    Command::JoinShared {
        url: url.to_string(),
        branch: String::new(),
    }
}

#[test]
fn a_repo_that_reads_as_a_marketplace_is_named_and_written() {
    let (home, spawner, services, mut state) = fresh();
    let url = origin(home.path(), "acme");
    send(&mut state, &services, &spawner, join(&url));
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    let wanted = SharedConfig {
        url: url.clone(),
        branch: "main".into(),
        enabled: Vec::new(),
    };
    assert_eq!(
        state.config.shared(),
        Some(&wanted),
        "main when no branch is named"
    );
    let copy = state.skill.shared.as_ref().expect("the copy");
    assert_eq!(copy.marketplace.name, "groove-agent");
    assert_eq!(copy.marketplace.plugins[0].name, "acme");
    let written = groove_config_service::load(&home.path().join("config")).unwrap();
    assert_eq!(
        written.and_then(|one| one.shared),
        Some(wanted),
        "the file holds it"
    );
}

#[test]
fn a_repo_that_is_no_marketplace_is_refused_and_not_written() {
    let (home, spawner, services, mut state) = fresh();
    let url = origin(home.path(), "groove");
    send(&mut state, &services, &spawner, join(&url));
    assert_eq!(state.config.shared(), None);
    assert!(state.skill.shared.is_none());
    let why = state
        .skill
        .unshared
        .clone()
        .expect("the refusal is kept for Settings");
    assert!(why.contains("groove"), "{why}");
}

#[test]
fn letting_the_repo_go_forgets_it_and_its_copy() {
    let (home, spawner, services, mut state) = fresh();
    let url = origin(home.path(), "acme");
    send(&mut state, &services, &spawner, join(&url));
    let copy = state.skill.shared.clone().expect("the copy").path;
    send(&mut state, &services, &spawner, Command::LeaveShared);
    assert_eq!(state.config.shared(), None);
    assert!(state.skill.shared.is_none());
    assert!(!copy.exists(), "the copy is gone");
}

#[test]
fn a_listing_out_when_the_repo_is_let_go_brings_neither_it_nor_its_copy_back() {
    let (home, spawner, services, mut state) = fresh();
    let url = origin(home.path(), "acme");
    send(&mut state, &services, &spawner, join(&url));
    let copy = state.skill.shared.clone().expect("the copy").path;
    let late = SyncSpawner::new().unwrap();
    let list = Cmd::Agent(crate::agent::Command::ListSkills);
    dispatch(list, &mut state, &services, &late);
    send(&mut state, &services, &spawner, Command::LeaveShared);
    until(&late, &services, &mut state, |_| late.idle());
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(state.skill.shared.is_none(), "the older listing is dropped");
    assert!(!copy.exists(), "the copy is gone");
}

#[test]
fn a_join_asked_while_a_listing_is_out_copies_once_the_listing_lands() {
    let (home, spawner, services, mut state) = fresh();
    let url = origin(home.path(), "acme");
    let late = SyncSpawner::new().unwrap();
    let list = Cmd::Agent(crate::agent::Command::ListSkills);
    dispatch(list, &mut state, &services, &late);
    dispatch(Cmd::Config(join(&url)), &mut state, &services, &spawner);
    let copy = home.path().join("data/shared");
    assert!(!copy.exists(), "nothing copied while the listing is out");
    until(&late, &services, &mut state, |_| late.idle());
    until(&spawner, &services, &mut state, |_| spawner.idle());
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(
        state.skill.shared.is_some() && copy.exists(),
        "copied after"
    );
}

#[test]
fn the_copy_stays_while_the_config_without_it_cannot_be_written() {
    let (home, spawner, services, mut state) = fresh();
    let url = origin(home.path(), "acme");
    send(&mut state, &services, &spawner, join(&url));
    let copy = state.skill.shared.clone().expect("the copy").path;
    std::fs::create_dir(home.path().join("config/config.json.tmp")).unwrap();
    send(&mut state, &services, &spawner, Command::LeaveShared);
    assert_eq!(state.errors.len(), 1, "the failed write is said");
    assert!(copy.exists(), "the copy the file still names stays");
}

#[test]
fn a_follow_that_keeps_failing_is_said_once() {
    let (_home, _spawner, _services, mut state) = fresh();
    let failed = || Some(Err(groove_types::Error::invalid("origin does not answer")));
    crate::config::shared::followed(&mut state, failed());
    crate::config::shared::followed(&mut state, failed());
    assert_eq!(state.errors.len(), 1, "the same failure is not said again");
}

#[test]
fn an_enabled_shared_skill_is_listed_on_and_given_to_a_launch() {
    let (home, spawner, services, mut state) = fresh();
    let url = origin(home.path(), "platform");
    send(&mut state, &services, &spawner, join(&url));
    let skill = |state: &AppState| {
        let found = state
            .skill
            .skills
            .iter()
            .find(|one| one.id == "platform:rollout");
        found.map(|one| one.enabled)
    };
    assert_eq!(skill(&state), Some(false), "listed, and off until enabled");

    if let Some(shared) = state
        .config
        .config
        .as_mut()
        .and_then(|one| one.shared.as_mut())
    {
        shared.enabled = vec!["platform:rollout".into()];
    }
    let list = crate::agent::Command::ListSkills;
    dispatch(Cmd::Agent(list), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |_| spawner.idle());
    assert_eq!(skill(&state), Some(true));
    let dirs = crate::agent::skills::dirs(&state);
    let dirs = spawner
        .block_on(groove_skill_service::skills::plugin_dirs(&dirs))
        .unwrap();
    assert!(
        dirs.iter()
            .any(|one| one.ends_with("plugins/shared/platform")),
        "{dirs:?}"
    );
}

fn switch(id: &str, on: bool, state: &mut AppState, services: &Services, spawner: &SyncSpawner) {
    let id = id.to_string();
    dispatch(
        Cmd::Agent(crate::agent::Command::SwitchSkill { id, on }),
        state,
        services,
        spawner,
    );
    until(spawner, services, state, |_| spawner.idle());
}

#[test]
fn a_skill_of_the_users_switched_off_is_written_listed_off_and_marks_agents_stale() {
    let (home, spawner, services, mut state) = fresh();
    let dirs = crate::agent::skills::dirs(&state);
    spawner
        .block_on(groove_skill_service::skills::sync(&dirs))
        .unwrap();
    let saved = groove_skill_service::skills::save(&dirs, "ship-it", "---\n---\nGo.\n", None);
    spawner.block_on(saved).unwrap();
    let before = groove_types::Timestamp::now();
    switch("user:ship-it", false, &mut state, &services, &spawner);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert_eq!(state.config.skills_off(), ["user:ship-it"]);
    let written = groove_config_service::load(&home.path().join("config")).unwrap();
    assert_eq!(
        written.map(|one| one.skills_off),
        Some(vec!["user:ship-it".into()])
    );
    let held = state
        .skill
        .skills
        .iter()
        .find(|one| one.id == "user:ship-it");
    assert_eq!(held.map(|one| one.enabled), Some(false));
    assert!(
        state.skill.switched >= before,
        "a running agent reads as stale"
    );

    switch("user:ship-it", true, &mut state, &services, &spawner);
    assert!(state.config.skills_off().is_empty(), "on again");
}

#[test]
fn a_core_skill_cannot_be_switched_off() {
    let (_home, spawner, services, mut state) = fresh();
    switch("groove:save-task", false, &mut state, &services, &spawner);
    assert_eq!(state.errors.len(), 1, "refused");
    assert!(state.config.skills_off().is_empty());
}

#[test]
fn a_listing_out_when_a_skill_is_switched_off_never_switches_it_back_on() {
    let (_home, spawner, services, mut state) = fresh();
    let dirs = crate::agent::skills::dirs(&state);
    spawner
        .block_on(groove_skill_service::skills::sync(&dirs))
        .unwrap();
    let saved = groove_skill_service::skills::save(&dirs, "ship-it", "---\n---\nGo.\n", None);
    spawner.block_on(saved).unwrap();
    let late = SyncSpawner::new().unwrap();
    let list = Cmd::Agent(crate::agent::Command::ListSkills);
    dispatch(list, &mut state, &services, &late);
    switch("user:ship-it", false, &mut state, &services, &spawner);
    until(&late, &services, &mut state, |_| late.idle());
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    let held = state
        .skill
        .skills
        .iter()
        .find(|one| one.id == "user:ship-it");
    assert_eq!(
        held.map(|one| one.enabled),
        Some(false),
        "the older listing is dropped"
    );
}

#[test]
fn before_the_first_run_a_join_is_refused_and_nothing_is_copied() {
    let (home, spawner, services, mut state) = fresh();
    let url = origin(home.path(), "acme");
    state.config.config = None;
    send(&mut state, &services, &spawner, join(&url));
    assert_eq!(state.errors.len(), 1, "the join is refused");
    assert!(!home.path().join("data/shared").exists(), "nothing cloned");
}

#[test]
fn a_listing_asked_while_a_refused_copy_is_out_still_runs() {
    let (home, spawner, services, mut state) = fresh();
    let url = origin(home.path(), "acme");
    let held = crate::tests::fixture::Held::default();
    dispatch(Cmd::Config(join(&url)), &mut state, &services, &held);
    let list = Cmd::Agent(crate::agent::Command::ListSkills);
    dispatch(list, &mut state, &services, &spawner);
    assert!(!state.skill.listed, "the listing waits on the copy");
    state.config.config = None;
    for job in held.take() {
        spawner.block_on(job)(&mut state, &services, &spawner);
    }
    until(&spawner, &services, &mut state, |_| spawner.idle());
    assert_eq!(state.errors.len(), 1, "the copy is refused");
    assert!(state.skill.listed, "the listing asked meanwhile ran");
}

#[test]
fn a_repo_joined_again_before_the_leave_lands_keeps_its_copy() {
    let (home, spawner, services, mut state) = fresh();
    let url = origin(home.path(), "acme");
    send(&mut state, &services, &spawner, join(&url));
    let copy = state.skill.shared.clone().expect("the copy").path;
    let held = crate::tests::fixture::Held::default();
    let leave = Cmd::Config(Command::LeaveShared);
    dispatch(leave, &mut state, &services, &held);
    send(&mut state, &services, &spawner, join(&url));
    for job in held.take() {
        spawner.block_on(job)(&mut state, &services, &spawner);
    }
    assert!(copy.exists(), "the copy the config names is not taken away");
    until(&spawner, &services, &mut state, |_| spawner.idle());
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(state.config.shared().is_some(), "joined again");
}

#[test]
fn a_skill_read_that_fails_keeps_the_last_list_and_says_why() {
    let (_home, spawner, services, mut state) = fresh();
    let list = Cmd::Agent(crate::agent::Command::ListSkills);
    dispatch(list, &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |_| spawner.idle());
    let before = state.skill.skills.clone();
    assert!(!before.is_empty(), "the core skills are listed");
    let failed = Err(groove_types::Error::invalid("the read panicked"));
    crate::agent::skills::relisted(&mut state, failed);
    assert_eq!(state.skill.skills, before, "the last list is kept");
    assert_eq!(state.errors.len(), 1, "and the failure is said");
}

#[test]
fn a_leave_whose_write_failed_forgets_the_copy_once_a_later_write_lands() {
    let (home, spawner, services, mut state) = fresh();
    let url = origin(home.path(), "acme");
    send(&mut state, &services, &spawner, join(&url));
    let copy = state.skill.shared.clone().expect("the copy").path;
    let blocker = home.path().join("config/config.json.tmp");
    std::fs::create_dir(&blocker).unwrap();
    send(&mut state, &services, &spawner, Command::LeaveShared);
    assert!(copy.exists(), "the file still names it");
    std::fs::remove_dir(&blocker).unwrap();
    let preference = groove_config_service::Preference::AutoApproveDefault(true);
    send(
        &mut state,
        &services,
        &spawner,
        Command::SetPreference(preference),
    );
    until(&spawner, &services, &mut state, |_| spawner.idle());
    assert!(
        !copy.exists(),
        "the copy goes with the leave that landed late"
    );
}
