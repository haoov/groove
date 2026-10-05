use groove_types::{ExternalId, SessionKind};

use crate::{Dirs, delete, is_name, list, plugin_dirs, read, save, sync};

/// Both plugin directories under a temporary home.
fn dirs(home: &std::path::Path) -> Dirs {
    Dirs::new(&home.join("data"), &home.join("config"))
}

fn named(skills: &[groove_types::Skill]) -> Vec<&str> {
    skills.iter().map(|one| one.id.as_str()).collect()
}

#[test]
fn the_core_plugin_is_written_from_the_ones_compiled_in() {
    let home = tempfile::tempdir().unwrap();
    let dirs = dirs(home.path());
    sync(&dirs).expect("both plugins");

    let skills = list(&dirs);
    let ids = named(&skills);
    assert!(ids.contains(&"groove:save-task"), "{ids:?}");
    assert!(ids.contains(&"groove:co-review"), "{ids:?}");
    assert!(ids.contains(&"groove:promote-fact"), "{ids:?}");
    assert_eq!(ids.len(), 10,"every core skill, and no user one yet");

    let one = skills
        .iter()
        .find(|one| one.id == "groove:save-task")
        .expect("the skill");
    assert_eq!(one.label, "save task");
    assert_eq!(one.hint, "Commit, push, MR, and update the task.");
    assert_eq!(one.kinds, ["task"]);
    assert!(!one.editable, "a core skill is not the user's to write");
    assert!(one.changed_at.seconds() > 0, "it knows when it was written");
}

#[test]
fn a_skill_the_core_plugin_lost_goes_with_the_next_start() {
    let home = tempfile::tempdir().unwrap();
    let dirs = dirs(home.path());
    sync(&dirs).expect("both plugins");
    let stale = dirs.core.join("skills").join("old-one");
    std::fs::create_dir_all(&stale).unwrap();
    std::fs::write(stale.join("SKILL.md"), "---\n---\n").unwrap();

    sync(&dirs).expect("written again");
    assert!(!stale.exists(), "what is not compiled in does not stay");
}

#[test]
fn a_skill_of_the_users_own_is_written_read_and_deleted() {
    let home = tempfile::tempdir().unwrap();
    let dirs = dirs(home.path());
    sync(&dirs).expect("both plugins");
    let body = "---\nname: ship-it\ndescription: Ship it.\ngroove-label: ship it\n---\n\nDo it.\n";

    let made = save(&dirs, "ship-it", body, None).expect("the skill is written");
    assert_eq!(made.id, "user:ship-it");
    assert_eq!(made.label, "ship it");
    assert!(made.editable);
    assert_eq!(read(&dirs, "user:ship-it").expect("its own file"), body);
    assert!(named(&list(&dirs)).contains(&"user:ship-it"));

    delete(&dirs, "ship-it").expect("it goes");
    assert!(!named(&list(&dirs)).contains(&"user:ship-it"));
}

#[test]
fn a_name_that_is_already_taken_is_refused_unless_it_is_the_one_replaced() {
    let home = tempfile::tempdir().unwrap();
    let dirs = dirs(home.path());
    sync(&dirs).expect("both plugins");
    let body = "---\ndescription: One.\n---\n\nOne.\n";
    save(&dirs, "ship-it", body, None).expect("the first");

    let again = save(&dirs, "ship-it", body, None);
    assert!(again.is_err(), "it does not overwrite by accident");
    save(&dirs, "ship-it", body, Some("ship-it")).expect("naming it replaces it");

    save(&dirs, "ship-them", body, Some("ship-it")).expect("renamed");
    assert!(
        !named(&list(&dirs)).contains(&"user:ship-it"),
        "the old name goes"
    );
}

#[test]
fn a_name_that_is_not_a_path_segment_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let dirs = dirs(home.path());
    sync(&dirs).expect("both plugins");
    for bad in ["../escape", "Ship It", "", "-leading"] {
        assert!(!is_name(bad), "{bad}");
        assert!(save(&dirs, bad, "body", None).is_err(), "{bad}");
        assert!(delete(&dirs, bad).is_err(), "{bad}");
    }
    assert!(read(&dirs, "user:../escape").is_err());
    assert!(read(&dirs, "nothing:save-task").is_err());
    assert!(read(&dirs, "save-task").is_err(), "an id names its plugin");
}

#[test]
fn a_skill_says_which_kinds_offer_it() {
    let home = tempfile::tempdir().unwrap();
    let dirs = dirs(home.path());
    sync(&dirs).expect("both plugins");
    let skills = list(&dirs);
    let review = SessionKind::Review {
        project: "g/mayo".into(),
        iid: 7,
    };
    let task = SessionKind::Task {
        external_id: ExternalId::new("gh#1"),
    };

    let save = skills.iter().find(|one| one.name == "save-task").unwrap();
    assert!(save.offered_to(&task));
    assert!(!save.offered_to(&review), "a review has no task to land");

    let co = skills.iter().find(|one| one.name == "co-review").unwrap();
    assert!(co.offered_to(&review));
}

#[test]
fn a_plugin_with_no_skill_is_not_handed_to_the_launch() {
    let home = tempfile::tempdir().unwrap();
    let dirs = dirs(home.path());
    sync(&dirs).expect("both plugins");
    assert_eq!(
        plugin_dirs(&dirs),
        vec![dirs.core.clone()],
        "none of theirs yet"
    );

    save(&dirs, "ship-it", "---\n---\n\nGo.\n", None).expect("one of their own");
    assert_eq!(
        plugin_dirs(&dirs),
        vec![dirs.core.clone(), dirs.mine.clone()]
    );
}

#[test]
fn a_skill_of_the_users_switched_off_is_listed_off_and_not_given() {
    let home = tempfile::tempdir().unwrap();
    let dirs = dirs(home.path()).switched(vec!["user:quiet".into()]);
    sync(&dirs).unwrap();
    save(&dirs, "quiet", "---\n---\n\nShh.\n", None).unwrap();
    save(&dirs, "loud", "---\n---\n\nHey.\n", None).unwrap();
    let mine: Vec<(String, bool)> = list(&dirs)
        .into_iter()
        .filter(|one| one.plugin == "user")
        .map(|one| (one.id, one.enabled))
        .collect();
    assert_eq!(
        mine,
        [("user:loud".into(), true), ("user:quiet".into(), false)]
    );
    assert!(dirs.mine.join("skills/loud").exists());
    assert!(
        !dirs.mine.join("skills/quiet").exists(),
        "not given to a launch"
    );
}

#[test]
fn filing_a_task_is_offered_everywhere_and_converting_only_to_an_explorer() {
    let home = tempfile::tempdir().unwrap();
    let dirs = dirs(home.path());
    sync(&dirs).expect("both plugins");
    let skills = list(&dirs);
    let skill = |id: &str| skills.iter().find(|one| one.id == id).expect(id);
    let task = SessionKind::Task {
        external_id: ExternalId::new("gh/haoov/groove#50"),
    };

    let create = skill("groove:create-task");
    assert!(create.offered_to(&task), "a task files a follow-up");
    assert!(create.offered_to(&SessionKind::Explorer));

    let convert = skill("groove:convert-explorer");
    assert!(convert.offered_to(&SessionKind::Explorer));
    assert!(!convert.offered_to(&task), "a task is one already");
}

/// A marketplace listing `entries`, each plugin's own manifest naming it `own`.
fn listing(home: &std::path::Path, entries: &[(&str, &str, &str)]) -> std::path::PathBuf {
    let repo = home.join("shared");
    std::fs::create_dir_all(repo.join(".claude-plugin")).unwrap();
    let mut listed = Vec::new();
    for (name, source, own) in entries {
        let dir = repo
            .join(source.trim_start_matches("./"))
            .join(".claude-plugin");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.json"), format!(r#"{{ "name": "{own}" }}"#)).unwrap();
        listed.push(serde_json::json!({ "name": name, "source": source }));
    }
    let manifest = serde_json::json!({ "name": "groove-agent", "owner": {}, "plugins": listed });
    std::fs::write(
        repo.join(".claude-plugin").join("marketplace.json"),
        manifest.to_string(),
    )
    .unwrap();
    repo
}

#[test]
fn a_shared_repo_reads_as_the_plugins_its_marketplace_lists() {
    let home = tempfile::tempdir().unwrap();
    let repo = listing(
        home.path(),
        &[
            ("platform", "./plugins/platform", "platform"),
            ("review", "./plugins/review", "review"),
        ],
    );
    let read = crate::marketplace(&repo).unwrap();
    assert_eq!(read.name, "groove-agent");
    let names: Vec<&str> = read.plugins.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names, ["platform", "review"]);
    assert_eq!(read.plugins[0].dir, repo.join("plugins/platform"));
}

#[test]
fn a_plugin_groove_cannot_read_or_name_is_refused() {
    let refused = |entry: (&str, &str, &str)| {
        let home = tempfile::tempdir().unwrap();
        crate::marketplace(&listing(home.path(), &[entry])).is_err()
    };
    assert!(
        refused(("groove", "./plugins/groove", "groove")),
        "a name Groove keeps"
    );
    assert!(
        refused(("user", "./plugins/user", "user")),
        "a name Groove keeps"
    );
    assert!(
        refused(("platform", "./plugins/platform", "other")),
        "named otherwise inside"
    );
    assert!(
        refused(("platform", "./../platform", "platform")),
        "outside the repo"
    );
    assert!(
        refused(("Team X", "./plugins/x", "Team X")),
        "no name of the kind"
    );
    let home = tempfile::tempdir().unwrap();
    assert!(
        crate::marketplace(home.path()).is_err(),
        "no marketplace at all"
    );
}

/// A shared plugin `platform` holding `skills`, read into the dirs with `enabled` on.
fn sharing(home: &std::path::Path, skills: &[&str], enabled: &[&str]) -> Dirs {
    let repo = listing(home, &[("platform", "./plugins/platform", "platform")]);
    for name in skills {
        let at = repo.join("plugins/platform/skills").join(name);
        std::fs::create_dir_all(&at).unwrap();
        let body = format!("---\ndescription: {name} things\n---\nDo {name}.\n");
        std::fs::write(at.join("SKILL.md"), body).unwrap();
    }
    let plugins = crate::marketplace(&repo).unwrap().plugins;
    let enabled = enabled.iter().map(|one| one.to_string()).collect();
    dirs(home).sharing(plugins, enabled)
}

#[test]
fn shared_skills_stand_between_the_core_ones_and_the_users_and_start_off() {
    let home = tempfile::tempdir().unwrap();
    let dirs = sharing(home.path(), &["rollout", "triage"], &["platform:triage"]);
    sync(&dirs).unwrap();
    save(&dirs, "mine", "---\ndescription: mine\n---\nMine.\n", None).unwrap();
    let skills = list(&dirs);
    let shared: Vec<(&str, bool)> = skills
        .iter()
        .filter(|one| one.plugin == "platform")
        .map(|one| (one.id.as_str(), one.enabled))
        .collect();
    assert_eq!(
        shared,
        [("platform:rollout", false), ("platform:triage", true)]
    );
    let plugins: Vec<&str> = skills.iter().map(|one| one.plugin.as_str()).collect();
    let (core, user) = (
        plugins.iter().rposition(|one| *one == "groove"),
        plugins.iter().position(|one| *one == "user"),
    );
    let first = plugins.iter().position(|one| *one == "platform");
    assert!(core < first && first < user, "{plugins:?}");
}

#[test]
fn a_launch_is_given_the_enabled_shared_skills_alone() {
    let home = tempfile::tempdir().unwrap();
    let dirs = sharing(home.path(), &["rollout", "triage"], &["platform:triage"]);
    sync(&dirs).unwrap();
    let built = dirs.built.join("platform");
    assert!(plugin_dirs(&dirs).contains(&built));
    let names: Vec<String> = std::fs::read_dir(built.join("skills"))
        .unwrap()
        .map(|one| one.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["triage"], "only the enabled one");
    let manifest = std::fs::read_to_string(built.join(".claude-plugin/plugin.json")).unwrap();
    assert!(
        manifest.contains("\"platform\""),
        "named as the shared plugin: {manifest}"
    );
}

#[test]
fn a_shared_plugin_with_nothing_enabled_is_not_given_at_all() {
    let home = tempfile::tempdir().unwrap();
    let dirs = sharing(home.path(), &["rollout"], &["platform:triage"]);
    sync(&dirs).unwrap();
    assert!(
        !plugin_dirs(&dirs)
            .iter()
            .any(|one| one.starts_with(&dirs.built))
    );
    let off = dirs.clone().sharing(dirs.shared.clone(), Vec::new());
    sync(&off).unwrap();
    assert!(
        !dirs.built.join("platform").exists(),
        "a skill switched off goes at the next build"
    );
}

#[test]
fn a_shared_skill_is_read_from_the_copy() {
    let home = tempfile::tempdir().unwrap();
    let dirs = sharing(home.path(), &["rollout"], &[]);
    assert!(
        read(&dirs, "platform:rollout")
            .unwrap()
            .contains("Do rollout.")
    );
}

#[test]
fn a_core_skill_that_did_not_change_keeps_the_time_it_was_written() {
    let home = tempfile::tempdir().unwrap();
    let dirs = dirs(home.path());
    sync(&dirs).unwrap();
    let file = home
        .path()
        .join("data/plugins/groove/skills/save-task/SKILL.md");
    let past = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000);
    std::fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(past)
        .unwrap();
    sync(&dirs).unwrap();
    let skills = list(&dirs);
    let one = skills
        .iter()
        .find(|one| one.id == "groove:save-task")
        .unwrap();
    assert_eq!(one.changed_at.seconds(), 1_000);
}
