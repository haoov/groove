use std::path::{Path, PathBuf};

use groove_types::{Caret, Diagnostic, Error, LanguageServer, Severity, WorktreeId};

use crate::{ServerKey, Servers, Tell, Telling, serving};

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
