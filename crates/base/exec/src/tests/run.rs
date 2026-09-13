use std::time::Duration;

use crate::Error;
use crate::run::*;

fn sh(script: &str) -> Run {
    Run::new("sh").args(["-c", script])
}

#[tokio::test]
async fn output_carries_status_stdout_and_stderr() {
    let out = sh("echo out; echo err >&2; exit 3").output().await.unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(out.stdout, b"out\n");
    assert_eq!(out.stderr, b"err\n");
}

#[tokio::test]
async fn text_returns_stdout_on_success() {
    assert_eq!(sh("printf hello").text().await.unwrap(), "hello");
}

#[tokio::test]
async fn text_reports_redacted_stderr_on_failure() {
    let err = sh("echo 'https://user:secret@host/p' >&2; exit 1")
        .text()
        .await
        .unwrap_err();
    let message = err.to_string();
    assert!(message.contains("<redacted>@host/p"), "{message}");
    assert!(!message.contains("secret"), "{message}");
}

#[tokio::test]
async fn a_slow_child_times_out() {
    let err = sh("sleep 5")
        .timeout(Duration::from_millis(50))
        .output()
        .await
        .unwrap_err();
    assert!(matches!(err, Error::TimedOut { .. }), "{err}");
}

#[tokio::test]
async fn a_missing_program_fails_to_spawn() {
    let err = Run::new("groove-no-such-program")
        .output()
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Spawn { .. }), "{err}");
}

#[tokio::test]
async fn cwd_and_env_reach_the_child() {
    let out = sh("pwd; printf %s \"$GROOVE_X\"")
        .cwd("/")
        .env("GROOVE_X", "y")
        .text()
        .await
        .unwrap();
    assert_eq!(out, "/\ny");
}
