use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use groove_types::{Session, SessionId, SessionKind, Timestamp};

use crate::prompt::core_prompt;
use crate::session::session_file;
use crate::session::session_uuid;
use crate::{Launch, Loopback, Paths};

pub(crate) fn explorer() -> Session {
    Session {
        id: SessionId::new("explorer-ab12cd34"),
        title: "try the new grid".into(),
        kind: SessionKind::Explorer,
        created_at: Timestamp::new(0),
    }
}

pub(crate) fn paths<'a>(root: &'a Path) -> Paths<'a> {
    Paths {
        home: root,
        launch_dir: root,
        cwd: root,
    }
}

pub(crate) fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == flag)
        .map(|i| args[i + 1].as_str())
}

#[test]
fn the_session_uuid_is_stable_and_v5() {
    let a = session_uuid("gh-groove-50");
    assert_eq!(a, session_uuid("gh-groove-50"));
    assert_ne!(a, session_uuid("gh-groove-51"));
    assert_eq!(a.len(), 36);
    assert_eq!(&a[14..15], "5", "version nibble");
}

#[test]
fn the_session_file_encodes_the_cwd_like_claude_code() {
    let file = session_file(
        Path::new("/home/x"),
        Path::new("/home/x/worktrees/gitlab.wiremind.io/devops/"),
        "u",
    );
    assert_eq!(
        file,
        PathBuf::from(
            "/home/x/.claude/projects/-home-x-worktrees-gitlab-wiremind-io-devops/u.jsonl"
        )
    );
}

#[test]
fn a_first_launch_names_the_session_and_a_second_resumes_it() {
    let root = tempfile::tempdir().unwrap();
    let session = explorer();
    let first = Launch::plan(&session, &paths(root.path()), &[], None).unwrap();
    let uuid = session_uuid(session.id.as_str());
    assert_eq!(
        &first.args[..2],
        &["--session-id".to_string(), uuid.clone()]
    );

    let file = session_file(root.path(), root.path(), &uuid);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "").unwrap();
    let second = Launch::plan(&session, &paths(root.path()), &[], None).unwrap();
    assert_eq!(&second.args[..2], &["--resume".to_string(), uuid]);
}

#[test]
fn a_session_handed_an_explorer_s_conversation_resumes_it() {
    let root = tempfile::tempdir().unwrap();
    let explorer = explorer();
    let uuid = session_uuid(explorer.id.as_str());
    let file = session_file(root.path(), root.path(), &uuid);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "").unwrap();

    let task = Session {
        id: SessionId::new("gh-haoov-groove-50"),
        title: "Harden Groove".into(),
        kind: SessionKind::Task {
            external_id: groove_types::ExternalId::new("github.com/haoov/groove#50"),
        },
        created_at: Timestamp::new(0),
    };
    crate::hand_over(root.path(), explorer.id.as_str(), task.id.as_str()).unwrap();
    let launch = Launch::plan(&task, &paths(root.path()), &[], None).unwrap();
    assert_eq!(
        &launch.args[..2],
        &["--resume".to_string(), uuid],
        "the task carries on the explorer's conversation"
    );
}

#[test]
fn forgetting_a_session_takes_its_launch_files_and_no_other() {
    let root = tempfile::tempdir().unwrap();
    Launch::plan(&explorer(), &paths(root.path()), &[], None).unwrap();
    let explorer = explorer().id;
    crate::hand_over(root.path(), explorer.as_str(), "gh-haoov-groove-50").unwrap();
    crate::forget(root.path(), explorer.as_str()).unwrap();
    crate::forget(root.path(), explorer.as_str()).unwrap();
    let left: Vec<String> = std::fs::read_dir(root.path())
        .unwrap()
        .map(|one| one.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.'))
        .collect();
    assert_eq!(left, ["gh-haoov-groove-50.thread"]);
}

#[test]
fn the_prompt_file_is_private_and_names_the_session() {
    let root = tempfile::tempdir().unwrap();
    let launch = Launch::plan(
        &explorer(),
        &paths(root.path()),
        &[PathBuf::from("/p/groove")],
        None,
    )
    .unwrap();
    let prompt = flag_value(&launch.args, "--append-system-prompt-file").unwrap();
    assert!(prompt.ends_with("explorer-ab12cd34.prompt.md"));
    let mode = std::fs::metadata(prompt).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let text = std::fs::read_to_string(prompt).unwrap();
    assert!(text.starts_with(
        "You are the Groove agent for session explorer-ab12cd34 (explorer): \"try the new grid\"."
    ));
    assert_eq!(flag_value(&launch.args, "--plugin-dir"), Some("/p/groove"));
    assert!(launch.args.iter().all(|a| a != "--mcp-config"));
    assert_eq!(launch.cwd, root.path());
    assert!(
        launch
            .env
            .iter()
            .any(|(k, v)| k == "MCP_TOOL_TIMEOUT" && v == "86400000")
    );
}

#[test]
fn the_token_reaches_the_files_and_never_the_command_line() {
    let root = tempfile::tempdir().unwrap();
    let loopback = Loopback {
        tools: Some(crate::Tools {
            sse_url: "http://127.0.0.1:41823/sse?task=explorer-ab12cd34".into(),
            token: "t00l-token".into(),
        }),
        hook_url: "http://127.0.0.1:27413/hook/explorer-ab12cd34".into(),
        token: "s3cr3t-token".into(),
    };
    let launch = Launch::plan(&explorer(), &paths(root.path()), &[], Some(&loopback)).unwrap();
    assert!(
        launch
            .args
            .iter()
            .all(|a| !a.contains("s3cr3t") && !a.contains("t00l")),
        "{:?}",
        launch.args
    );
    let mcp = std::fs::read_to_string(flag_value(&launch.args, "--mcp-config").unwrap()).unwrap();
    assert!(mcp.contains("Bearer t00l-token"));
    assert!(
        !mcp.contains("s3cr3t"),
        "the hooks token stays with the hooks"
    );
    let settings =
        std::fs::read_to_string(flag_value(&launch.args, "--settings").unwrap()).unwrap();
    assert!(
        !settings.contains("s3cr3t"),
        "the hook command is visible in ps"
    );
    assert!(settings.contains("-K '"));
    assert!(settings.contains("\"Stop\""));
    let curl = std::fs::read_to_string(root.path().join("explorer-ab12cd34.hooks.curl")).unwrap();
    assert!(curl.contains("s3cr3t-token"));
}

#[test]
fn the_core_prompt_keeps_its_rules() {
    let text = core_prompt(&explorer());
    assert!(text.contains("A write waits for a human"));
    assert!(text.contains("Never write in a clone"));
    assert!(!text.contains("{{"));
}

#[test]
fn hooks_run_without_the_tool_server() {
    let root = tempfile::tempdir().unwrap();
    let loopback = Loopback {
        tools: None,
        hook_url: "http://127.0.0.1:27413/hook/explorer-ab12cd34".into(),
        token: "s3cr3t-token".into(),
    };
    let launch = Launch::plan(&explorer(), &paths(root.path()), &[], Some(&loopback)).unwrap();
    assert!(launch.args.iter().all(|a| a != "--mcp-config"));
    let settings =
        std::fs::read_to_string(flag_value(&launch.args, "--settings").unwrap()).unwrap();
    assert!(settings.contains("/hook/explorer-ab12cd34"));
}
