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
    assert_eq!(ids.len(), 8, "every core skill, and no user one yet");

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
        vec![dirs.core.clone(), dirs.user.clone()]
    );
}
