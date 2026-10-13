use groove_types::Attached;

use crate::check;

fn held(context: &str, namespace: Option<&str>) -> Attached {
    Attached {
        context: context.into(),
        namespace: namespace.map(String::from),
    }
}

fn args(line: &str) -> Vec<String> {
    line.split(' ').map(String::from).collect()
}

/// The arguments kubectl runs with, or why the read is refused.
fn asked(context: &str, line: &str, attached: &[Attached]) -> Result<Vec<String>, String> {
    check(context, &args(line), attached)
        .map(|one| one.argv)
        .map_err(|e| e.message)
}

#[test]
fn a_read_runs_on_the_context_it_names_with_its_own_arguments() {
    let whole = [held("prod", None)];
    let argv = asked("prod", "get pods -n api -o wide", &whole).unwrap();
    assert_eq!(argv, args("--context prod get pods -n api -o wide"));
}

#[test]
fn a_context_the_session_does_not_hold_is_refused() {
    let why = asked("staging", "get pods", &[held("prod", None)]).unwrap_err();
    assert!(why.contains("no context staging"), "{why}");
    assert!(why.contains("prod"), "{why}");
}

#[test]
fn only_reads_are_run() {
    let whole = [held("prod", None)];
    for line in [
        "delete pod api-0",
        "exec api-0 -- sh",
        "apply",
        "rollout restart deploy/api",
        "edit cm/x",
        "proxy",
        "foo",
    ] {
        assert!(asked("prod", line, &whole).is_err(), "{line}");
    }
    let why = asked("prod", "delete pod api-0", &whole).unwrap_err();
    assert!(why.contains("the reads are"), "{why}");
    for line in [
        "get pods",
        "describe pod api-0",
        "logs api-0",
        "events",
        "top pods",
        "explain pods",
        "auth can-i get pods",
        "rollout history deploy/api",
        "version",
    ] {
        assert!(asked("prod", line, &whole).is_ok(), "{line}");
    }
}

#[test]
fn a_flag_no_read_takes_is_refused() {
    let whole = [held("prod", None)];
    for line in [
        "get pods --server=https://evil",
        "get pods --kubeconfig /tmp/k",
        "get pods --token abc",
        "get pods --as admin",
        "get pods --context other",
        "get --raw /api/v1/secrets",
        "get pods -w",
        "logs api-0 -f",
        "get -f pod.yaml",
        "get pods -v=9",
        "get pods --template={{.}}",
    ] {
        let why = asked("prod", line, &whole).unwrap_err();
        assert!(why.contains("not a flag a read takes"), "{line}: {why}");
    }
}

#[test]
fn a_bare_dash_or_double_dash_is_refused() {
    let whole = [held("prod", None)];
    assert!(asked("prod", "logs api-0 --", &whole).is_err());
    assert!(asked("prod", "get -", &whole).is_err());
}

#[test]
fn a_flag_value_written_in_any_form_is_read() {
    let whole = [held("prod", None)];
    for line in [
        "get pods -oyaml",
        "get pods -o=yaml",
        "get pods --output=yaml",
        "get pods -o yaml",
    ] {
        assert!(asked("prod", line, &whole).is_ok(), "{line}");
    }
    assert!(
        asked("prod", "get pods -n", &whole)
            .unwrap_err()
            .contains("needs a value")
    );
}

#[test]
fn a_namespace_must_be_held_unless_the_whole_cluster_is() {
    let some = [held("prod", Some("api")), held("prod", Some("web"))];
    assert!(asked("prod", "get pods -n api", &some).is_ok());
    let why = asked("prod", "get pods -n kube-system", &some).unwrap_err();
    assert!(
        why.contains("kube-system") && why.contains("api, web"),
        "{why}"
    );
    let why = asked("prod", "get pods -A", &some).unwrap_err();
    assert!(why.contains("whole cluster"), "{why}");
    let why = asked("prod", "get pods", &some).unwrap_err();
    assert!(why.contains("-n"), "{why}");
    let whole = [held("prod", None)];
    assert!(asked("prod", "get pods -A", &whole).is_ok());
    assert!(asked("prod", "get pods -n anything", &whole).is_ok());
}

#[test]
fn the_one_namespace_held_is_named_when_the_read_names_none() {
    let one = [held("prod", Some("api"))];
    let argv = asked("prod", "get pods", &one).unwrap();
    assert_eq!(argv, args("--context prod get pods --namespace api"));
}

#[test]
fn a_read_of_no_namespace_needs_none() {
    let some = [held("prod", Some("api")), held("prod", Some("web"))];
    for line in ["api-resources", "explain deployments", "version"] {
        let argv = asked("prod", line, &some).unwrap();
        assert!(!argv.contains(&"--namespace".to_string()), "{line}");
    }
}

#[test]
fn a_secret_s_data_is_never_printed() {
    let whole = [held("prod", None)];
    for line in [
        "get secret db -o yaml",
        "get secrets -o json",
        "get secrets/db -ojsonpath={.data}",
        "get pods,secrets -o yaml",
        "get Secret.v1 db -o yaml",
        "get secret db -o custom-columns=D:.data",
    ] {
        let why = asked("prod", line, &whole).unwrap_err();
        assert!(why.contains("secret's data"), "{line}: {why}");
    }
    for line in [
        "get secrets",
        "get secrets -o wide",
        "get secret -o name",
        "describe secret db",
        "get pods -o yaml",
    ] {
        assert!(asked("prod", line, &whole).is_ok(), "{line}");
    }
}

#[test]
fn a_log_read_with_no_bound_reads_its_last_lines() {
    let whole = [held("prod", None)];
    let argv = asked("prod", "logs api-0", &whole).unwrap();
    assert_eq!(argv.last().map(String::as_str), Some("--tail=200"));
    for line in ["logs api-0 --tail=50", "logs api-0 --since=10m"] {
        let argv = asked("prod", line, &whole).unwrap();
        assert!(!argv.contains(&"--tail=200".to_string()), "{line}");
    }
}

#[test]
fn a_rollout_status_never_waits() {
    let whole = [held("prod", None)];
    let argv = asked("prod", "rollout status deploy/api", &whole).unwrap();
    assert_eq!(argv.last().map(String::as_str), Some("--watch=false"));
}

#[test]
fn a_write_is_refused_for_its_verb_before_its_flags() {
    let whole = [held("prod", None)];
    let why = asked("prod", "scale deployment/api --replicas=1 -n api", &whole).unwrap_err();
    assert!(why.starts_with("scale is not a read"), "{why}");
    let why = asked("prod", "rollout restart deployment/api", &whole).unwrap_err();
    assert!(why.starts_with("rollout restart is not a read"), "{why}");
    let why = asked("prod", "get pods --replicas=1", &whole).unwrap_err();
    assert!(
        why.contains("--replicas is not a flag a read takes"),
        "{why}"
    );
}

#[test]
fn a_template_is_never_read_from_a_file() {
    let whole = [held("prod", None)];
    for line in [
        "get pods -o jsonpath-file=/etc/passwd",
        "get pods --output=go-template-file=/x",
        "get pods -ocustom-columns-file=/x",
        "get pods -o template-file=/x",
    ] {
        let why = asked("prod", line, &whole).unwrap_err();
        assert!(why.contains("names no file"), "{line}: {why}");
    }
    assert!(
        asked(
            "prod",
            "get pods -o jsonpath={.items[*].metadata.name}",
            &whole
        )
        .is_ok()
    );
}

#[test]
fn a_secret_s_metadata_is_read_by_named_field_and_nothing_else() {
    let whole = [held("prod", None)];
    for line in [
        "get secret db -o jsonpath={.metadata.managedFields}",
        "get secret db -o jsonpath={.metadata.name}{\"\\n\"}{.metadata.creationTimestamp}",
        "get secret db -o jsonpath={.metadata.annotations.reconcile\\.external-secrets\\.io/data-hash}",
        "get secret db -o jsonpath={.metadata.annotations['external-secrets.io/force-sync']}",
        "get secrets --sort-by=.metadata.creationTimestamp",
    ] {
        assert!(asked("prod", line, &whole).is_ok(), "{line}");
    }
    let listed = [
        "get",
        "secrets",
        "-o",
        "jsonpath={range .items[*]}{.metadata.name}{\" \"}{.type}{\"\\n\"}{end}",
    ];
    let listed: Vec<String> = listed.iter().map(|one| one.to_string()).collect();
    assert!(check("prod", &listed, &whole).is_ok());
    for line in [
        "get secret db -o jsonpath={.metadata}",
        "get secret db -o jsonpath={.metadata.annotations}",
        "get secret db -o jsonpath={.metadata.annotations.kubectl\\.kubernetes\\.io/last-applied-configuration}",
        "get secret db -o jsonpath={..data}",
        "get secret db -o jsonpath={.metadata.name}{.data.password}",
        "get secrets -o jsonpath={.items[?(@.data)]}",
        "get secrets --sort-by=.data.password",
        "get secret db -o custom-columns=N:.metadata.name",
    ] {
        let why = asked("prod", line, &whole).unwrap_err();
        assert!(why.contains("secret's data"), "{line}: {why}");
    }
}

#[test]
fn a_flag_of_letters_past_ascii_is_refused_not_a_crash() {
    let whole = [held("prod", None)];
    for line in ["get pods -é", "get pods -oé", "get pods -é=x"] {
        let _ = asked("prod", line, &whole);
    }
    let why = asked("prod", "get pods -éa", &whole).unwrap_err();
    assert!(why.contains("-éa"), "{why}");
}
