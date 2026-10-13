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

#[tokio::test]
async fn input_reaches_the_child() {
    let out = Run::new("cat").input("what it reads").text().await;
    assert_eq!(out.expect("cat ran"), "what it reads");
}

#[tokio::test]
async fn an_input_larger_than_the_pipes_is_read_back_whole() {
    let big = "a line the child answers as it reads\n".repeat(110_000);
    let out = Run::new("cat")
        .input(big.clone())
        .timeout(std::time::Duration::from_secs(10))
        .text()
        .await;
    assert_eq!(out.expect("cat ran to its end").len(), big.len());
}

#[tokio::test]
async fn a_child_that_exits_without_reading_still_says_why() {
    let big = "unread\n".repeat(200_000);
    let out = Run::new("sh")
        .args(["-c", "echo refused >&2; exit 3"])
        .input(big)
        .text()
        .await;
    let said = out.expect_err("it exits 3").to_string();
    assert!(said.ends_with("failed: refused"), "{said}");
}

#[tokio::test]
async fn a_capped_run_stops_reading_at_its_limit_and_says_it_cut() {
    let started = std::time::Instant::now();
    let ran = sh("yes groove; echo never >&2").timeout(Duration::from_secs(10));
    let ran = ran.capped(1000).await.unwrap();
    assert_eq!(ran.stdout.len(), 1000);
    assert!(ran.cut && !ran.success);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the child ends at the cut"
    );
}

#[tokio::test]
async fn a_capped_run_under_its_limit_keeps_all_and_its_stderr() {
    let ran = sh("echo out; echo err >&2; exit 1")
        .capped(1000)
        .await
        .unwrap();
    assert_eq!(ran.stdout, b"out\n");
    assert_eq!(ran.stderr, "err");
    assert!(!ran.cut && !ran.success);
}

#[tokio::test]
async fn a_bare_run_sees_only_the_environment_it_is_given() {
    let ran = sh("echo \"$HOME|$ONE\"").bare().env("ONE", "1");
    let ran = ran.input("unread").capped(1000).await.unwrap();
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "|1\n");
}
