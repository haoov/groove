use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::Error;
use crate::piped::*;

fn sh(script: &str) -> Piped {
    Piped::new("sh").args(["-c", script])
}

async fn all_of(mut process: Process) -> String {
    let Pipes {
        stdin, mut stdout, ..
    } = process.pipes().unwrap();
    drop(stdin);
    let mut out = String::new();
    stdout.read_to_string(&mut out).await.unwrap();
    process.wait().await.unwrap();
    out
}

#[tokio::test]
async fn a_piped_child_answers_each_line_it_reads() {
    let mut process = Piped::new("cat").start().unwrap();
    let Pipes {
        mut stdin, stdout, ..
    } = process.pipes().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    for word in ["one", "two"] {
        stdin
            .write_all(format!("{word}\n").as_bytes())
            .await
            .unwrap();
        stdin.flush().await.unwrap();
        assert_eq!(lines.next_line().await.unwrap().as_deref(), Some(word));
    }
}

#[tokio::test]
async fn the_child_writes_its_log_on_stderr() {
    let mut process = sh("echo said >&2").start().unwrap();
    let Pipes { mut stderr, .. } = process.pipes().unwrap();
    let mut err = String::new();
    stderr.read_to_string(&mut err).await.unwrap();
    assert_eq!(err, "said\n");
}

#[tokio::test]
async fn the_pipes_are_taken_once() {
    let mut process = sh("true").start().unwrap();
    assert!(process.pipes().is_some());
    assert!(process.pipes().is_none());
}

#[tokio::test]
async fn an_unset_variable_never_reaches_the_child() {
    let process = sh("printf %s \"${HOME-gone}\"")
        .unset("HOME")
        .start()
        .unwrap();
    assert_eq!(all_of(process).await, "gone");
}

#[tokio::test]
async fn the_child_runs_in_its_cwd_with_its_env() {
    let process = sh("pwd; printf %s \"$GROOVE_X\"")
        .cwd("/")
        .env("GROOVE_X", "y")
        .start()
        .unwrap();
    assert_eq!(all_of(process).await, "/\ny");
}

#[tokio::test]
async fn a_missing_program_fails_to_start() {
    let Err(err) = Piped::new("groove-no-such-program").start() else {
        panic!("a missing program started");
    };
    assert!(matches!(err, Error::Spawn { .. }), "{err}");
}

#[tokio::test]
async fn terminate_ends_the_child() {
    let mut process = sh("sleep 30").start().unwrap();
    process.terminate().unwrap();
    let code = tokio::time::timeout(Duration::from_secs(5), process.wait())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(code, None);
}

#[tokio::test]
async fn dropping_the_process_kills_the_child() {
    let process = sh("sleep 30").start().unwrap();
    let pid = nix::unistd::Pid::from_raw(process.pid().unwrap() as i32);
    drop(process);
    for _ in 0..100 {
        if nix::sys::signal::kill(pid, None).is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the child outlived its process");
}
