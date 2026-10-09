use groove_types::State;

use crate::describe;

fn crashing() -> serde_json::Value {
    serde_json::json!({
        "metadata": { "name": "worker-0", "namespace": "paxone", "uid": "u1",
            "creationTimestamp": "2026-10-08T10:01:00Z",
            "labels": { "app": "worker" }, "annotations": { "checksum/config": "abc" },
            "ownerReferences": [{ "apiVersion": "apps/v1", "kind": "ReplicaSet", "name": "worker-5d6b9c7f4" }] },
        "spec": {
            "nodeName": "gra9-node-2", "serviceAccountName": "paxone", "priority": 0,
            "imagePullSecrets": [{ "name": "registry" }],
            "volumes": [
                { "name": "tls", "secret": { "secretName": "paxone-tls" } },
                { "name": "kube-api-access-x", "projected": { "sources": [{ "configMap": { "name": "kube-root-ca.crt" } }] } }
            ],
            "initContainers": [{ "name": "migrate", "image": "backend:4.21.3" }],
            "containers": [{ "name": "worker", "image": "backend:4.21.3",
                "ports": [{ "name": "http", "containerPort": 8080 }],
                "env": [{ "name": "DB", "valueFrom": { "secretKeyRef": { "name": "paxone-db-legacy", "key": "url" } } }],
                "envFrom": [{ "configMapRef": { "name": "paxone-env" } }],
                "livenessProbe": { "httpGet": { "path": "/healthz", "port": 8080 }, "periodSeconds": 10 },
                "volumeMounts": [{ "name": "tls", "mountPath": "/etc/tls", "readOnly": true },
                                 { "name": "kube-api-access-x", "mountPath": "/var/run/secrets" }],
                "resources": { "requests": { "cpu": "250m", "memory": "512Mi" }, "limits": { "cpu": "1" } } }]
        },
        "status": {
            "phase": "Running", "podIP": "10.2.4.118", "qosClass": "Burstable", "startTime": "2026-10-08T10:01:00Z",
            "conditions": [{ "type": "Ready", "status": "False", "reason": "ContainersNotReady",
                             "lastTransitionTime": "2026-10-08T10:40:00Z" }],
            "initContainerStatuses": [{ "name": "migrate", "ready": true, "restartCount": 0,
                "state": { "terminated": { "exitCode": 0, "reason": "Completed" } } }],
            "containerStatuses": [{ "name": "worker", "ready": false, "restartCount": 14,
                "state": { "waiting": { "reason": "CrashLoopBackOff" } },
                "lastState": { "terminated": { "exitCode": 1, "reason": "Error", "finishedAt": "2026-10-08T10:40:00Z" } } }]
        }
    })
}

#[test]
fn a_crashing_pod_reads_as_its_tab_draws_it() {
    let pod = describe("Pod", &crashing(), true);
    assert_eq!(
        (pod.name.as_str(), pod.namespace.as_deref()),
        ("worker-0", Some("paxone"))
    );
    assert_eq!(pod.owners[0].group(), "apps");
    assert_eq!(pod.owners[0].kind, "ReplicaSet");
    let part = pod.pod.expect("a pod part");
    assert_eq!(part.status, "CrashLoopBackOff");
    assert_eq!(part.priority, Some(("default".into(), 0)));
    assert!(!part.conditions[0].met);
    let uses: Vec<(&str, &str)> = part
        .uses
        .iter()
        .map(|(k, n)| (k.as_str(), n.as_str()))
        .collect();
    assert_eq!(
        uses,
        [
            ("sa", "paxone"),
            ("secret", "paxone-tls"),
            ("cm", "paxone-env"),
            ("secret", "paxone-db-legacy"),
            ("secret", "registry"),
            ("node", "gra9-node-2")
        ]
    );
    let (init, worker) = (&part.containers[0], &part.containers[1]);
    assert!(init.init && matches!(&init.state, State::Terminated { exit: 0, .. }));
    assert_eq!(
        worker.state,
        State::Waiting {
            reason: "CrashLoopBackOff".into()
        }
    );
    assert_eq!(
        (worker.restarts, worker.last.as_ref().map(|one| one.exit)),
        (14, Some(1))
    );
    assert_eq!(worker.ports, ["http 8080/TCP"]);
    assert_eq!(
        worker.liveness.as_deref(),
        Some("GET :8080/healthz · every 10s")
    );
    assert_eq!(worker.mounts, ["tls → /etc/tls ro"]);
    assert_eq!(worker.cpu, (Some("250m".into()), Some("1".into())));
    assert_eq!(worker.memory, (Some("512Mi".into()), None));
    assert!(pod.yaml.contains("restartCount: 14"));
}

#[test]
fn an_init_container_that_fails_names_the_pod_s_status() {
    let mut pod = crashing();
    pod["status"]["initContainerStatuses"][0]["state"] =
        serde_json::json!({ "terminated": { "exitCode": 2, "reason": "Error" } });
    assert_eq!(
        describe("Pod", &pod, true).pod.expect("a pod part").status,
        "Init:Error"
    );
}

#[test]
fn an_event_a_workload_and_a_service_read_their_own_parts() {
    let event = serde_json::json!({ "metadata": { "name": "e1", "uid": "e1" }, "type": "Warning",
        "reason": "BackOff", "count": 187, "message": "Back-off restarting\n",
        "lastTimestamp": "2026-10-08T10:40:00Z",
        "involvedObject": { "kind": "Pod", "name": "worker-0" } });
    let row = describe("Event", &event, true).event.expect("an event row");
    assert!(row.warning);
    assert_eq!(
        (row.count, row.message.as_str(), row.about.as_str()),
        (187, "Back-off restarting", "pod/worker-0")
    );
    let deployment = serde_json::json!({ "metadata": { "name": "worker" },
        "spec": { "replicas": 3 }, "status": { "readyReplicas": 2 } });
    assert_eq!(
        describe("Deployment", &deployment, true).replicas,
        Some((2, 3))
    );
    let service = serde_json::json!({ "metadata": { "name": "metrics" }, "spec": { "selector": { "app": "worker" } } });
    assert_eq!(
        describe("Service", &service, true).selector,
        [("app".to_string(), "worker".to_string())]
    );
}

#[test]
fn a_whole_object_reads_as_yaml_and_a_part_read_has_none() {
    assert!(
        describe("Pod", &crashing(), true)
            .yaml
            .contains("nodeName: gra9-node-2")
    );
    assert!(describe("Pod", &crashing(), false).yaml.is_empty());
}
