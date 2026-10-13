use crate::{Checked, LIMIT, run};

fn checked(argv: &[&str]) -> Checked {
    Checked {
        verb: "get".into(),
        argv: argv.iter().map(|one| one.to_string()).collect(),
    }
}

#[tokio::test]
async fn the_read_runs_as_its_arguments_and_answers_its_output() {
    let answer = run(
        "echo",
        &checked(&["--context", "prod", "get", "pods"]),
        None,
    )
    .await
    .unwrap();
    assert_eq!(answer.text, "--context prod get pods\n");
    assert!(!answer.failed);
}

#[tokio::test]
async fn a_refused_read_answers_what_kubectl_said() {
    let script = checked(&[
        "-c",
        "echo 'Error from server (NotFound): pods \"x\" not found' >&2; exit 1",
    ]);
    let answer = run("sh", &script, None).await.unwrap();
    assert!(answer.failed);
    assert!(answer.text.contains("NotFound"), "{}", answer.text);
}

#[tokio::test]
async fn an_answer_past_the_limit_is_cut_and_says_so() {
    let answer = run("sh", &checked(&["-c", "yes groove"]), None)
        .await
        .unwrap();
    assert!(answer.text.len() < LIMIT + 200);
    assert!(
        answer.text.contains("cut at 32 KiB"),
        "the read is told to narrow"
    );
    assert!(!answer.failed);
}

#[tokio::test]
async fn a_jq_filter_reads_the_json() {
    let json = checked(&["-c", r#"echo '{"items":[{"name":"api"},{"name":"web"}]}'"#]);
    let answer = run("sh", &json, Some(".items[].name")).await.unwrap();
    assert_eq!(answer.text, "\"api\"\n\"web\"\n");
}

#[tokio::test]
async fn a_jq_filter_reads_nothing_of_groove_s_environment() {
    let json = checked(&["-c", "echo '{}'"]);
    let answer = run("sh", &json, Some("$ENV | keys")).await.unwrap();
    let keys: Vec<&str> = answer.text.split('"').skip(1).step_by(2).collect();
    assert!(keys.iter().all(|one| *one == "PATH"), "{keys:?}");
}

#[tokio::test]
async fn a_jq_filter_that_looks_like_a_flag_is_refused() {
    let json = checked(&["-c", "echo '{}'"]);
    assert!(run("sh", &json, Some("--rawfile")).await.is_err());
}

#[tokio::test]
async fn a_jq_filter_loads_no_module() {
    let json = checked(&["-c", "echo '{}'"]);
    for filter in [r#"import "x" as $x; ."#, r#"include "x"; ."#] {
        assert!(run("sh", &json, Some(filter)).await.is_err(), "{filter}");
    }
    let named = checked(&["-c", r#"echo '{"importance":1}'"#]);
    let answer = run("sh", &named, Some(".importance")).await.unwrap();
    assert_eq!(answer.text, "1\n");
}
