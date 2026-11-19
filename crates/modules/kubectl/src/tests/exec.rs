use crate::{Exec, RUNS, exec};

fn psql(command: &[&str]) -> Exec {
    Exec {
        context: "prod".into(),
        namespace: "paxone".into(),
        pod: "db-0".into(),
        container: Some("postgres".into()),
        command: command.iter().map(|one| one.to_string()).collect(),
        timeout: RUNS,
    }
}

#[tokio::test]
async fn a_command_runs_in_its_pod_s_container_after_a_double_dash() {
    let asked = psql(&["psql", "-c", "select 1"]);
    let answer = exec("echo", &asked).await.unwrap();
    let said = "--context prod --namespace paxone exec db-0 --container postgres \
                -- psql -c select 1\n";
    assert_eq!(answer.text, said);
    assert!(!answer.failed);
}

#[test]
fn a_command_shows_as_a_shell_reads_it_and_an_empty_one_is_refused() {
    let one = psql(&["psql", "-c", "select 'a'"]);
    assert_eq!(one.shown(), r"psql -c 'select '\''a'\'''");
    assert!(psql(&[]).argv().is_err());
    let flagged = Exec {
        pod: "--all".into(),
        ..psql(&["ls"])
    };
    assert!(flagged.argv().is_err(), "a name is no flag");
}

#[tokio::test]
async fn a_command_that_exits_non_zero_is_a_failure() {
    let answer = exec("false", &psql(&["psql"])).await.unwrap();
    assert!(answer.failed);
}

#[test]
fn a_shell_in_a_pod_takes_the_keys_on_a_terminal() {
    let shell = psql(&crate::SHELL);
    let argv = shell.terminal().unwrap();
    let said = argv.join(" ");
    let wanted = "exec db-0 --stdin --tty --container postgres -- sh -c";
    assert!(said.contains(wanted), "{said}");
    assert!(!shell.argv().unwrap().contains(&"--tty".to_string()));
}
