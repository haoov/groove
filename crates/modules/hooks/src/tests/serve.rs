use std::sync::Arc;
use std::sync::mpsc::{Receiver as Channel, channel};

use groove_types::HookKind;

use crate::{Post, Receiver, serve};

/// A receiver on a real port, and the posts it lets through.
fn listening(handle: &tokio::runtime::Handle) -> (Receiver, Channel<Post>) {
    let (sender, posts) = channel();
    let sender = Arc::new(std::sync::Mutex::new(sender));
    let receiver = serve(handle, move |post| {
        if let Ok(sender) = sender.lock() {
            let _ = sender.send(post);
        }
    })
    .expect("a loopback port");
    (receiver, posts)
}

async fn post(receiver: &Receiver, token: &str, body: &str) -> reqwest::StatusCode {
    reqwest::Client::new()
        .post(receiver.hook_url("gh-groove-50"))
        .bearer_auth(token)
        .body(body.to_string())
        .send()
        .await
        .expect("the receiver answers")
        .status()
}

#[test]
fn a_hook_with_the_token_reaches_the_app() {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let (receiver, posts) = listening(runtime.handle());
    let body = r#"{"hook_event_name":"UserPromptSubmit"}"#;
    let status = runtime.block_on(post(&receiver, &receiver.token, body));
    assert_eq!(status, 204);
    let post = posts.recv().expect("the post arrived");
    assert_eq!(post.session.as_str(), "gh-groove-50");
    assert_eq!(post.kind, HookKind::UserPromptSubmit);
}

#[test]
fn a_hook_without_the_token_is_refused_and_never_arrives() {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let (receiver, posts) = listening(runtime.handle());
    let body = r#"{"hook_event_name":"Stop"}"#;
    let status = runtime.block_on(post(&receiver, "not-the-token", body));
    assert_eq!(status, 401);
    assert!(posts.try_recv().is_err(), "nothing reached the app");
}

#[test]
fn a_body_that_is_not_a_hook_is_refused() {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let (receiver, _posts) = listening(runtime.handle());
    let status = runtime.block_on(post(&receiver, &receiver.token, "{}"));
    assert_eq!(status, 400);
}
