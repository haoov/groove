//! The team's shared repo: named once it reads as a marketplace, refused when not, let go.

use std::path::Path;

use groove_types::SharedConfig;

use crate::config::Command;
use crate::tests::fixture::{fresh, sh};
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

fn send(command: Command, state: &mut AppState, services: &Services, spawner: &SyncSpawner) {
    dispatch(Cmd::Config(command), state, services, spawner);
    for _ in 0..4 {
        spawner.drain(state, services);
    }
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
    let url = origin(home.path(), "wiremind");
    send(join(&url), &mut state, &services, &spawner);
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
    let copy = state.agent.shared.as_ref().expect("the copy");
    assert_eq!(copy.marketplace.name, "groove-agent");
    assert_eq!(copy.marketplace.plugins[0].name, "wiremind");
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
    send(join(&url), &mut state, &services, &spawner);
    assert_eq!(state.config.shared(), None);
    assert!(state.agent.shared.is_none());
    let why = state
        .config
        .unshared
        .clone()
        .expect("the refusal is kept for Settings");
    assert!(why.contains("groove"), "{why}");
}

#[test]
fn letting_the_repo_go_forgets_it_and_its_copy() {
    let (home, spawner, services, mut state) = fresh();
    let url = origin(home.path(), "wiremind");
    send(join(&url), &mut state, &services, &spawner);
    let copy = state.agent.shared.clone().expect("the copy").path;
    send(Command::LeaveShared, &mut state, &services, &spawner);
    assert_eq!(state.config.shared(), None);
    assert!(state.agent.shared.is_none());
    assert!(!copy.exists(), "the copy is gone");
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
    send(join(&url), &mut state, &services, &spawner);
    let skill = |state: &AppState| {
        let found = state
            .agent
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
    for _ in 0..4 {
        spawner.drain(&mut state, &services);
    }
    assert_eq!(skill(&state), Some(true));
    let dirs = groove_agent_service::skills::plugin_dirs(&crate::agent::skills::dirs(&state));
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
    for _ in 0..4 {
        spawner.drain(state, services);
    }
}

#[test]
fn a_skill_of_the_users_switched_off_is_written_listed_off_and_marks_agents_stale() {
    let (home, spawner, services, mut state) = fresh();
    let dirs = crate::agent::skills::dirs(&state);
    groove_agent_service::skills::sync(&dirs).unwrap();
    groove_agent_service::skills::save(&dirs, "ship-it", "---\n---\nGo.\n", None).unwrap();
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
        .agent
        .skills
        .iter()
        .find(|one| one.id == "user:ship-it");
    assert_eq!(held.map(|one| one.enabled), Some(false));
    assert!(
        state.agent.switched >= before,
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
