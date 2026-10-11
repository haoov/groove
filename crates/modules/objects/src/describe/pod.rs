//! A pod's own part: its status as `kubectl get` says it, its facts, conditions, containers and uses.

use groove_types::{Condition, PodPart, Used};
use serde_json::Value;

use super::container::containers;
use super::{text, time};

/// Config maps every pod mounts for its API token, which say nothing about the pod.
const INJECTED: [&str; 1] = ["kube-root-ca.crt"];

pub(super) fn part(pod: &Value) -> PodPart {
    let spec = &pod["spec"];
    let status = &pod["status"];
    let priority = spec["priority"].as_i64().map(|value| {
        let class = spec["priorityClassName"].as_str().unwrap_or("default");
        (class.to_string(), value)
    });
    PodPart {
        status: said(pod),
        node: spec["nodeName"].as_str().map(String::from),
        ip: status["podIP"].as_str().map(String::from),
        qos: status["qosClass"].as_str().map(String::from),
        priority,
        started: time(status, "/startTime"),
        conditions: conditions(&status["conditions"]),
        containers: containers(pod),
        uses: uses(spec),
    }
}

/// The status `kubectl get pods` shows: a container's reason over the pod's phase.
fn said(pod: &Value) -> String {
    let status = &pod["status"];
    if pod.pointer("/metadata/deletionTimestamp").is_some() {
        return "Terminating".into();
    }
    let list = |at: &str| status[at].as_array().cloned().unwrap_or_default();
    for one in list("initContainerStatuses") {
        let exit = one
            .pointer("/state/terminated/exitCode")
            .and_then(Value::as_i64);
        if let Some(exit) = exit.filter(|code| *code != 0) {
            let reason = text(&one, "/state/terminated/reason");
            return format!(
                "Init:{}",
                if reason.is_empty() {
                    format!("ExitCode:{exit}")
                } else {
                    reason
                }
            );
        }
        let waiting = text(&one, "/state/waiting/reason");
        if !waiting.is_empty() && waiting != "PodInitializing" {
            return format!("Init:{waiting}");
        }
    }
    for one in list("containerStatuses").iter().rev() {
        for at in ["/state/waiting/reason", "/state/terminated/reason"] {
            let reason = text(one, at);
            if !reason.is_empty() {
                return reason;
            }
        }
    }
    let reason = text(status, "/reason");
    match reason.is_empty() {
        true => text(status, "/phase"),
        false => reason,
    }
}

fn conditions(list: &Value) -> Vec<Condition> {
    let list = list.as_array().map(Vec::as_slice).unwrap_or_default();
    list.iter()
        .map(|one| Condition {
            kind: text(one, "/type"),
            met: one["status"] == "True",
            reason: one["reason"].as_str().map(String::from),
            since: time(one, "/lastTransitionTime"),
        })
        .collect()
}

const IN_VOLUME: [(&str, &str); 3] = [
    ("cm", "/configMap/name"),
    ("secret", "/secret/secretName"),
    ("pvc", "/persistentVolumeClaim/claimName"),
];
const IN_SOURCE: [(&str, &str); 2] = [("cm", "/configMap/name"), ("secret", "/secret/name")];
const IN_FROM: [(&str, &str); 2] = [("cm", "/configMapRef/name"), ("secret", "/secretRef/name")];
const IN_ENV: [(&str, &str); 2] = [
    ("cm", "/valueFrom/configMapKeyRef/name"),
    ("secret", "/valueFrom/secretKeyRef/name"),
];

/// What the pod reads, each once, in the order the spec names it, with how it reads it.
fn uses(spec: &Value) -> Vec<Used> {
    let containers: Vec<&Value> = list(spec, "/initContainers")
        .iter()
        .chain(list(spec, "/containers"))
        .collect();
    let mut out: Vec<Used> = Vec::new();
    let account = spec["serviceAccountName"].as_str().into_iter();
    add(&mut out, account.map(|one| ("sa", one)), "service account");
    for volume in list(spec, "/volumes") {
        let how = mounted(&containers, &text(volume, "/name"));
        add(&mut out, named([volume].into_iter(), &IN_VOLUME), &how);
        let sources = list(volume, "/projected/sources").iter();
        add(&mut out, named(sources, &IN_SOURCE), &how);
    }
    for one in &containers {
        let how = format!("env {}", text(one, "/name"));
        add(
            &mut out,
            named(list(one, "/envFrom").iter(), &IN_FROM),
            &how,
        );
        add(&mut out, named(list(one, "/env").iter(), &IN_ENV), &how);
    }
    let pulls = list(spec, "/imagePullSecrets").iter();
    let pulls = pulls.filter_map(|one| Some(("secret", one["name"].as_str()?)));
    add(&mut out, pulls, "image pull");
    out
}

/// `mounted /etc/tls`, every path the containers mount `volume` at.
fn mounted(containers: &[&Value], volume: &str) -> String {
    let mut paths: Vec<String> = Vec::new();
    let mounts = containers.iter().flat_map(|one| list(one, "/volumeMounts"));
    for mount in mounts.filter(|one| text(one, "/name") == volume) {
        let path = text(mount, "/mountPath");
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    match paths.is_empty() {
        true => "volume".into(),
        false => format!("mounted {}", paths.join(", ")),
    }
}

/// Each object once; a second way of reading it joins the first.
fn add<'a>(out: &mut Vec<Used>, found: impl Iterator<Item = (&'a str, &'a str)>, how: &str) {
    for (tag, name) in found.filter(|(_, name)| !name.is_empty() && !INJECTED.contains(name)) {
        let held = out
            .iter_mut()
            .find(|one| one.tag == tag && one.name == name);
        match held {
            Some(one) if !one.how.iter().any(|said| said == how) => one.how.push(how.into()),
            Some(_) => {}
            None => out.push(Used {
                tag: tag.into(),
                name: name.into(),
                how: vec![how.into()],
            }),
        }
    }
}

fn list<'a>(value: &'a Value, at: &str) -> &'a [Value] {
    value
        .pointer(at)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// The names each item holds at the places `table` reads, with their kind.
fn named<'a>(
    items: impl Iterator<Item = &'a Value> + 'a,
    table: &'a [(&'a str, &'a str)],
) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
    items.flat_map(move |one| {
        table
            .iter()
            .filter_map(move |(kind, at)| Some((*kind, one.pointer(at)?.as_str()?)))
    })
}
