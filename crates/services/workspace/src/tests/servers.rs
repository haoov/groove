use std::path::{Path, PathBuf};

use groove_types::{Caret, Completion, Diagnostic, Error, LanguageServer, Severity, WorktreeId};

use crate::{ServerKey, Servers, Tell, Telling, matching, serving};

fn key(dir: &str) -> ServerKey {
    ServerKey {
        worktree: WorktreeId::new("w"),
        dir: PathBuf::from(dir),
        name: "ty".into(),
    }
}

fn opened(path: &str) -> Tell {
    Tell::Opened {
        path: path.into(),
        language: "python",
        text: "x\n".into(),
    }
}

fn found(message: &str, line: usize) -> Diagnostic {
    Diagnostic {
        start: Caret::new(line, 0),
        end: Caret::new(line, 1),
        severity: Severity::Error,
        message: message.into(),
        source: None,
        code: None,
    }
}

#[test]
fn the_first_open_starts_the_server_and_the_next_tells_wait() {
    let mut servers = Servers::default();
    let one = key("/w");
    assert_eq!(servers.tell(&one, opened("a.py")), Telling::Start);
    assert_eq!(servers.tell(&one, opened("b.py")), Telling::Told);
    assert!(servers.client(&one).is_none(), "not up before it started");
}

#[test]
fn only_an_open_starts_a_server() {
    let mut servers = Servers::default();
    let one = key("/w");
    let saved = Tell::Saved {
        path: "a.py".into(),
    };
    assert_eq!(servers.tell(&one, saved), Telling::Told);
    assert_eq!(servers.tell(&one, opened("a.py")), Telling::Start);
}

#[test]
fn a_server_that_failed_to_start_waits_for_the_next_open_to_try_again() {
    let mut servers = Servers::default();
    let one = key("/w");
    servers.tell(&one, opened("a.py"));
    assert!(
        servers
            .started(&one, Err(Error::not_found("no ty")))
            .is_err()
    );
    assert!(servers.is_down(&one));
    let typed = Tell::Changed {
        path: "a.py".into(),
        text: "y".into(),
    };
    assert_eq!(
        servers.tell(&one, typed),
        Telling::Down,
        "a keystroke starts nothing"
    );
    assert_eq!(servers.tell(&one, opened("b.py")), Telling::Start);
}

#[test]
fn a_worktree_moved_starts_its_server_again_where_it_now_is() {
    let mut servers = Servers::default();
    servers.tell(&key("/explorers/w"), opened("a.py"));
    servers
        .started(&key("/explorers/w"), Err(Error::not_found("no ty")))
        .ok();
    assert_eq!(
        servers.tell(&key("/tasks/w"), opened("a.py")),
        Telling::Start
    );
    assert!(
        !servers.is_down(&key("/explorers/w")),
        "the old one is gone"
    );
}

#[test]
fn diagnostics_are_kept_by_the_worktree_s_own_path() {
    let mut servers = Servers::default();
    let one = key("/w");
    servers.diagnosed(
        &one,
        Path::new("/w/src/a.py"),
        vec![found("late", 3), found("early", 1)],
    );
    servers.diagnosed(&one, Path::new("/elsewhere/b.py"), vec![found("out", 0)]);
    let read = servers.diagnostics(&one.worktree, "src/a.py");
    let messages: Vec<&str> = read.iter().map(|one| one.message.as_str()).collect();
    assert_eq!(messages, ["early", "late"]);
    assert!(
        servers
            .diagnostics(&one.worktree, "/elsewhere/b.py")
            .is_empty()
    );
}

#[test]
fn an_empty_publish_clears_the_file() {
    let mut servers = Servers::default();
    let one = key("/w");
    servers.diagnosed(&one, Path::new("/w/a.py"), vec![found("x", 0)]);
    servers.diagnosed(&one, Path::new("/w/a.py"), vec![]);
    assert!(servers.diagnostics(&one.worktree, "a.py").is_empty());
}

#[test]
fn a_closed_server_takes_its_diagnostics_and_leaves_the_others() {
    let mut servers = Servers::default();
    let (ty, mut ruff) = (key("/w"), key("/w"));
    ruff.name = "ruff".into();
    servers.diagnosed(&ty, Path::new("/w/a.py"), vec![found("ty", 0)]);
    servers.diagnosed(&ruff, Path::new("/w/a.py"), vec![found("ruff", 1)]);
    servers.closed(&ty);
    let left = servers.diagnostics(&ty.worktree, "a.py");
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].message, "ruff");
}

#[test]
fn a_forgotten_worktree_loses_its_servers_and_diagnostics() {
    let mut servers = Servers::default();
    let one = key("/w");
    servers.tell(&one, opened("a.py"));
    servers.diagnosed(&one, Path::new("/w/a.py"), vec![found("x", 0)]);
    servers.forget(&one.worktree);
    assert!(servers.diagnostics(&one.worktree, "a.py").is_empty());
    assert_eq!(servers.tell(&one, opened("a.py")), Telling::Start);
}

#[test]
fn the_defaults_serve_python_with_ty_and_yaml_with_its_server() {
    let defaults = LanguageServer::defaults();
    let python: Vec<&str> = serving(&defaults, "python")
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(python, ["ty"]);
    assert_eq!(
        serving(&defaults, "yaml")[0].program,
        "yaml-language-server"
    );
    assert!(serving(&defaults, "rust").is_empty());
    assert_eq!(defaults[0].unset, ["VIRTUAL_ENV"]);
    assert_eq!(
        defaults[0].watch,
        [".venv/lib/*/site-packages"],
        "ty sees what uv sync installs"
    );
}

#[test]
fn what_an_earlier_start_says_is_dropped_once_a_later_one_replaced_it() {
    let mut servers = Servers::default();
    let one = key("/w");
    servers.tell(&one, opened("a.py"));
    let first = servers.epoch(&one);
    servers.forget(&one.worktree);
    servers.tell(&one, opened("a.py"));
    servers.ended(&one, first);
    assert!(
        !servers.is_down(&one),
        "the restarted one is still starting"
    );
    servers.heard((&one, first), Path::new("/w/a.py"), vec![found("stale", 0)]);
    assert!(servers.diagnostics(&one.worktree, "a.py").is_empty());
    servers.heard(
        (&one, servers.epoch(&one)),
        Path::new("/w/a.py"),
        vec![found("fresh", 0)],
    );
    assert_eq!(
        servers.diagnostics(&one.worktree, "a.py")[0].message,
        "fresh"
    );
}

fn item(label: &str, sort: &str) -> Completion {
    Completion {
        label: label.into(),
        detail: None,
        insert: label.into(),
        range: None,
        filter: label.into(),
        sort: sort.into(),
    }
}

#[test]
fn an_answer_to_an_earlier_ask_is_dropped_and_the_last_items_stand_meanwhile() {
    let mut servers = Servers::default();
    let first = servers.offering("a.py", Caret::new(0, 1));
    let second = servers.offering("a.py", Caret::new(0, 2));
    servers.offers(first, vec![item("stale", "0")]);
    assert!(servers.offered.as_ref().unwrap().items.is_empty());
    servers.offers(second, vec![item("path", "0")]);
    servers.offering("a.py", Caret::new(0, 3));
    let offered = servers.offered.as_ref().unwrap();
    assert_eq!(
        (offered.items[0].label.as_str(), offered.at),
        ("path", Caret::new(0, 3))
    );
    servers.dismiss();
    servers.offers(second + 1, vec![item("late", "0")]);
    assert!(servers.offered.is_none(), "a dismissed offer stays shut");
}

#[test]
fn the_typed_word_keeps_what_holds_its_letters_those_it_begins_first() {
    let items = [
        item("PathLike", "0"),
        item("altsep", "1"),
        item("path", "2"),
    ];
    let kept: Vec<&str> = matching(&items, "pa")
        .iter()
        .map(|one| one.label.as_str())
        .collect();
    assert_eq!(kept, ["PathLike", "path"]);
    assert_eq!(matching(&items, "px").len(), 0);
    assert_eq!(matching(&items, "").len(), 3);
}

#[test]
fn a_line_reads_only_what_covers_it_and_the_file_keeps_its_counts() {
    let mut servers = Servers::default();
    let one = key("/w");
    let mut long = found("long", 2);
    long.end = Caret::new(6, 0);
    let mut warned = found("warned", 9);
    warned.severity = Severity::Warning;
    servers.diagnosed(
        &one,
        Path::new("/w/a.py"),
        vec![found("early", 0), long, found("late", 8), warned],
    );
    let on = |line| -> Vec<String> {
        let read = servers.diagnostics_on(&one.worktree, "a.py", line);
        let covering = read
            .iter()
            .filter(|one| one.start.line <= line && line <= one.end.line);
        covering.map(|one| one.message.clone()).collect()
    };
    assert_eq!(on(4), ["long"], "a range over several lines");
    assert_eq!(on(8), ["late"]);
    assert!(on(7).is_empty());
    assert_eq!(servers.tally(&one.worktree, "a.py"), (3, 1));
    servers.closed(&one);
    assert_eq!(servers.tally(&one.worktree, "a.py"), (0, 0));
}

#[test]
fn a_word_typed_asks_for_an_offer_and_another_edit_puts_it_away() {
    use crate::Offering;
    use groove_types::Edit;
    let mut servers = Servers::default();
    assert_eq!(servers.after(&Edit::Insert("x".into())), Offering::Ask);
    assert_eq!(servers.after(&Edit::Insert(".".into())), Offering::Ask);
    assert_eq!(
        servers.after(&Edit::Backspace),
        Offering::Keep,
        "no offer to narrow"
    );
    servers.offering("a.py", Caret::new(0, 1));
    assert_eq!(servers.after(&Edit::Backspace), Offering::Ask);
    assert_eq!(servers.after(&Edit::Newline), Offering::Dismiss);
    assert_eq!(
        servers.after(&Edit::Insert("xy".into())),
        Offering::Dismiss,
        "a paste"
    );
}
