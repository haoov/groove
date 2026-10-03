use groove_types::{RoutineKind, Trigger};

use crate::{Dirs, list, parse};

const FIX_CI: &str = "\
---
description: When CI goes red on a worktree, fix it.
skills: groove:fix-ci
kind: bound
on: ci-failed
---
Keep the fix to what the log blames.
";

#[test]
fn a_routine_says_its_skill_its_triggers_and_its_words() {
    let read = parse("user:fix-red-ci", "fix-red-ci", FIX_CI).unwrap();
    assert_eq!(read.skills, ["groove:fix-ci"]);
    assert_eq!(read.kind, RoutineKind::Bound);
    assert_eq!(read.on, [Trigger::CiFailed]);
    assert_eq!(read.words, "Keep the fix to what the log blames.");
    assert_eq!(read.description, "When CI goes red on a worktree, fix it.");
}

#[test]
fn a_standalone_routine_answers_to_the_app() {
    let text = "---\nskills: platform:check-tasks\nkind: standalone\non: daily, tasks-read\n---\n";
    let read = parse("shared:tasks", "tasks", text).unwrap();
    assert_eq!(read.on, [Trigger::Daily, Trigger::TasksRead]);
}

#[test]
fn a_file_that_mixes_the_kinds_says_why_it_is_no_routine() {
    let refused = |text: &str| parse("user:x", "x", text).unwrap_err();
    let head = "---\nskills: groove:fix-ci\n";
    assert!(refused(&format!("{head}kind: bound\non: daily\n---\n")).contains("daily"));
    assert!(refused(&format!("{head}kind: bound\n---\n")).contains("at least one"));
    let session = refused(&format!("{head}kind: standalone\non: ci-failed\n---\n"));
    assert!(session.contains("make it bound"), "{session}");
    assert!(refused(&format!("{head}kind: bound\non: soon\n---\n")).contains("soon"));
    assert!(refused("no front matter").contains("front matter"));
    assert!(refused("---\nkind: bound\non: ci-failed\n---\n").contains("asks nothing"));
    let two = "---\nskills: groove:fix-ci, groove:fix-notes\nkind: bound\non: ci-failed\n---\n";
    assert!(
        refused(two).contains("say what to do"),
        "several skills need words"
    );
}

#[test]
fn a_routine_may_use_several_skills_or_none_when_its_words_say_what_to_do() {
    let several = "---\nskills: groove:fix-ci, groove:fix-notes\nkind: bound\non: ci-failed\n---\n\
                   Fix the pipeline first, then the notes.\n";
    let read = parse("user:mend", "mend", several).unwrap();
    assert_eq!(read.skills, ["groove:fix-ci", "groove:fix-notes"]);
    let none = "---\nkind: standalone\non: daily\n---\nCheck every task has its properties.\n";
    assert!(parse("user:tidy", "tidy", none).unwrap().skills.is_empty());
}

#[test]
fn the_team_s_routines_stand_before_the_user_s_and_a_broken_file_is_listed_with_why() {
    let home = tempfile::tempdir().unwrap();
    let copy = home.path().join("copy");
    std::fs::create_dir_all(copy.join("routines")).unwrap();
    std::fs::create_dir_all(home.path().join("routines")).unwrap();
    std::fs::write(copy.join("routines/fix-red-ci.md"), FIX_CI).unwrap();
    std::fs::write(home.path().join("routines/broken.md"), "nothing").unwrap();
    std::fs::write(home.path().join("routines/notes.txt"), "not a routine").unwrap();
    let listed = list(&Dirs::new(home.path(), Some(&copy)));
    let ids: Vec<&str> = listed.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["shared:fix-red-ci", "user:broken"]);
    assert!(listed[0].read.is_ok());
    assert!(listed[1].read.is_err());
}

#[test]
fn an_action_routine_names_what_groove_does_and_needs_no_skill() {
    let text = "---\nkind: action\ndo: start-due\non: tasks-read\n---\n";
    let read = parse("user:start-due", "start-due", text).unwrap();
    assert_eq!(read.kind, RoutineKind::Action);
    assert_eq!(read.action, Some(groove_types::Action::StartDue));
    let refused = |text: &str| parse("user:x", "x", text).unwrap_err();
    assert!(refused("---\nkind: action\non: daily\n---\n").contains("`do`"));
    assert!(refused("---\nkind: action\ndo: dance\n---\n").contains("start-due"));
    let bound = "---\nskills: groove:fix-ci\nkind: bound\ndo: start-due\non: ci-failed\n---\n";
    assert!(refused(bound).contains("only an action"));
    assert!(refused("---\nkind: action\ndo: start-due\non: ci-failed\n---\n").contains("bound"));
}

#[test]
fn the_day_s_first_look_is_new_once_and_the_next_day_is_new_again() {
    let dir = tempfile::tempdir().unwrap();
    let at = dir.path().join("routines");
    assert!(crate::new_day(&at, "2026-10-03").unwrap());
    assert!(
        !crate::new_day(&at, "2026-10-03").unwrap(),
        "a restart the same day"
    );
    assert!(crate::new_day(&at, "2026-10-04").unwrap());
}

#[test]
fn a_run_asks_its_one_skill_alone_or_its_words_with_what_started_it() {
    let alone = parse("user:fix", "fix", FIX_CI).unwrap();
    let mut alone = alone;
    alone.words.clear();
    assert_eq!(
        crate::prompt(&alone, Some(Trigger::CiFailed), "CI failed on x"),
        "/groove:fix-ci CI failed on x"
    );
    let worded = parse("user:fix", "fix", FIX_CI).unwrap();
    let said = crate::prompt(&worded, None, "");
    assert!(
        said.starts_with("Routine `fix`, started by its button."),
        "{said}"
    );
    assert!(
        said.ends_with("Keep the fix to what the log blames."),
        "{said}"
    );
}
