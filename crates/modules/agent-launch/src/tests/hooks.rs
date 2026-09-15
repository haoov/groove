//! The hook command is a shell line inside a JSON file. Only running it proves it.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use groove_hooks::Post;
use groove_types::HookKind;

use crate::tests::launch::{explorer, flag_value, paths};
use crate::{Launch, Loopback};

/// A receiver on a real port, and the posts it lets through.
fn listening(handle: &tokio::runtime::Handle) -> (groove_hooks::Receiver, Receiver<Post>) {
    let (sender, posts) = channel();
    let sender = Arc::new(Mutex::new(sender));
    let receiver = groove_hooks::serve(handle, move |post| {
        if let Ok(sender) = sender.lock() {
            let _ = sender.send(post);
        }
    })
    .expect("a loopback port");
    (receiver, posts)
}

/// The command a hook runs, out of the settings file the launch wrote.
fn hook_command(root: &std::path::Path, loopback: &Loopback, event: &str) -> String {
    let launch = Launch::plan(&explorer(), &paths(root), &[], Some(loopback)).expect("a plan");
    let path = flag_value(&launch.args, "--settings").expect("the settings flag");
    let text = std::fs::read_to_string(path).expect("the settings file");
    let settings: serde_json::Value = serde_json::from_str(&text).expect("json");
    settings["hooks"][event][0]["hooks"][0]["command"]
        .as_str()
        .expect("a command")
        .to_string()
}

#[test]
fn the_hook_command_posts_the_payload_to_the_receiver() {
    if Command::new("curl").arg("--version").output().is_err() {
        return;
    }
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let (receiver, posts) = listening(runtime.handle());
    let root = tempfile::tempdir().expect("a temp dir");
    let loopback = Loopback {
        sse_url: None,
        hook_url: receiver.hook_url("gh-groove-50"),
        token: receiver.token.clone(),
    };
    let command = hook_command(root.path(), &loopback, "Stop");

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .stdin(Stdio::piped())
        .spawn()
        .expect("the hook runs");
    let payload = br#"{"hook_event_name":"Stop","session_id":"claude-uuid"}"#;
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(payload)
        .expect("the payload goes in");
    assert!(child.wait().expect("the hook ends").success());

    let post = posts
        .recv_timeout(Duration::from_secs(5))
        .expect("the post arrived");
    assert_eq!(post.session.as_str(), "gh-groove-50");
    assert_eq!(post.kind, HookKind::Stop);
}
