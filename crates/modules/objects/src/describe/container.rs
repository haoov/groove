//! A pod's containers, init ones first: their spec and their status joined by name.

use groove_types::{Container, Ended, State};
use serde_json::Value;

use super::{text, time};

pub(super) fn containers(pod: &Value) -> Vec<Container> {
    let list = |at: &str| {
        pod.pointer(at)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let mut out = Vec::new();
    for (init, specs, states) in [
        (
            true,
            "/spec/initContainers",
            "/status/initContainerStatuses",
        ),
        (false, "/spec/containers", "/status/containerStatuses"),
    ] {
        let states = list(states);
        for spec in list(specs) {
            let name = text(&spec, "/name");
            let status = states.iter().find(|one| one["name"] == name.as_str());
            out.push(container(&spec, status, init));
        }
    }
    out
}

fn container(spec: &Value, status: Option<&Value>, init: bool) -> Container {
    let resource = |at: &str| spec.pointer(at).and_then(Value::as_str).map(String::from);
    let status = status.cloned().unwrap_or(Value::Null);
    Container {
        name: text(spec, "/name"),
        image: text(spec, "/image"),
        init,
        state: state(&status["state"]),
        ready: status["ready"].as_bool().unwrap_or_default(),
        restarts: status["restartCount"].as_u64().unwrap_or_default() as u32,
        last: ended(&status["lastState"]["terminated"]),
        ports: ports(&spec["ports"]),
        liveness: probe(&spec["livenessProbe"]),
        readiness: probe(&spec["readinessProbe"]),
        mounts: mounts(&spec["volumeMounts"]),
        cpu: (
            resource("/resources/requests/cpu"),
            resource("/resources/limits/cpu"),
        ),
        memory: (
            resource("/resources/requests/memory"),
            resource("/resources/limits/memory"),
        ),
    }
}

fn state(state: &Value) -> State {
    if let Some(running) = state.get("running") {
        return State::Running {
            since: time(running, "/startedAt"),
        };
    }
    if let Some(waiting) = state.get("waiting") {
        return State::Waiting {
            reason: text(waiting, "/reason"),
        };
    }
    match ended(&state["terminated"]) {
        Some(ended) => State::Terminated {
            reason: ended.reason,
            exit: ended.exit,
        },
        None => State::Unknown,
    }
}

fn ended(terminated: &Value) -> Option<Ended> {
    let exit = terminated["exitCode"].as_i64()?;
    Some(Ended {
        reason: text(terminated, "/reason"),
        exit: exit as i32,
        at: time(terminated, "/finishedAt"),
    })
}

/// `http 8080/TCP`, or `8080/TCP` for a port with no name.
fn ports(list: &Value) -> Vec<String> {
    let list = list.as_array().map(Vec::as_slice).unwrap_or_default();
    list.iter()
        .map(|one| {
            let port = one["containerPort"].as_i64().unwrap_or_default();
            let protocol = one["protocol"].as_str().unwrap_or("TCP");
            match one["name"].as_str() {
                Some(name) => format!("{name} {port}/{protocol}"),
                None => format!("{port}/{protocol}"),
            }
        })
        .collect()
}

/// `GET :8080/healthz · every 10s`, `TCP :5432`, `exec pg_isready`.
fn probe(probe: &Value) -> Option<String> {
    let port = |at: &str| {
        probe
            .pointer(at)
            .map(|one| one.as_str().map_or_else(|| one.to_string(), String::from))
    };
    let how = if let Some(get) = probe.get("httpGet") {
        format!(
            "GET :{}{}",
            port("/httpGet/port")?,
            get["path"].as_str().unwrap_or("/")
        )
    } else if probe.get("tcpSocket").is_some() {
        format!("TCP :{}", port("/tcpSocket/port")?)
    } else if probe.get("grpc").is_some() {
        format!("gRPC :{}", port("/grpc/port")?)
    } else {
        let words = probe.pointer("/exec/command").and_then(Value::as_array)?;
        let words: Vec<&str> = words.iter().filter_map(Value::as_str).collect();
        format!("exec {}", words.join(" "))
    };
    let every = probe["periodSeconds"].as_i64().unwrap_or(10);
    Some(format!("{how} · every {every}s"))
}

/// `paxone-tls → /etc/tls ro`.
fn mounts(list: &Value) -> Vec<String> {
    let list = list.as_array().map(Vec::as_slice).unwrap_or_default();
    list.iter()
        .filter(|one| !text(one, "/name").starts_with("kube-api-access-"))
        .map(|one| {
            let only = if one["readOnly"] == true { " ro" } else { "" };
            format!("{} → {}{only}", text(one, "/name"), text(one, "/mountPath"))
        })
        .collect()
}
