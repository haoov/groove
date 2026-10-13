//! A read checked: its verb, its context and namespace held by the session, no secret's data.

use groove_types::{Attached, Error};

use crate::flags::{Read, read};

/// The reads the agent may ask, each by the words that name it.
const VERBS: &[&[&str]] = &[
    &["get"],
    &["describe"],
    &["logs"],
    &["events"],
    &["top"],
    &["explain"],
    &["api-resources"],
    &["api-versions"],
    &["version"],
    &["auth", "can-i"],
    &["rollout", "history"],
    &["rollout", "status"],
];

/// The reads that name no namespace.
const UNSCOPED: &[&str] = &["explain", "api-resources", "api-versions", "version"];

/// The outputs of a get that show no secret's data.
const PLAIN: &[&str] = &["wide", "name"];

/// The lines of a log read when the read names no bound.
const TAIL: &str = "--tail=200";

/// A read the session may run: its verb and the arguments kubectl takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    pub verb: String,
    pub argv: Vec<String>,
}

/// `args` read in `context`, checked against what the session holds.
pub fn check(context: &str, args: &[String], attached: &[Attached]) -> Result<Checked, Error> {
    let read = read(args).map_err(Error::invalid)?;
    let verb = verb(&read.words)?;
    let mut argv = vec!["--context".to_string(), context.to_string()];
    argv.extend(args.iter().cloned());
    if !UNSCOPED.contains(&verb.as_str()) {
        argv.extend(scoped(context, &read, attached)?);
    }
    if verb == "get" && shows_secrets(&read) {
        let why = "a secret's data is not read: get secrets without -o, or describe one";
        return Err(Error::invalid(why));
    }
    match verb.as_str() {
        "logs" if !bounded(&read) => argv.push(TAIL.to_string()),
        "rollout status" => argv.push("--watch=false".to_string()),
        _ => {}
    }
    Ok(Checked { verb, argv })
}

/// The read's verb, one of `VERBS`.
fn verb(words: &[String]) -> Result<String, Error> {
    let named = |one: &&&[&str]| one.iter().zip(words).all(|(at, word)| at == word);
    let known = VERBS
        .iter()
        .find(|one| one.len() <= words.len() && named(one));
    match known {
        Some(one) => Ok(one.join(" ")),
        None => {
            let listed: Vec<String> = VERBS.iter().map(|one| one.join(" ")).collect();
            let asked = words.first().map_or("nothing", String::as_str);
            let why = format!("{asked} is not a read; the reads are {}", listed.join(", "));
            Err(Error::invalid(why))
        }
    }
}

/// The namespace the read stands in, as the session holds it; what to add to say it.
fn scoped(context: &str, read: &Read, attached: &[Attached]) -> Result<Vec<String>, Error> {
    let held: Vec<&Attached> = attached
        .iter()
        .filter(|one| one.context == context)
        .collect();
    if held.is_empty() {
        let contexts: Vec<&str> = attached.iter().map(|one| one.context.as_str()).collect();
        let why = format!(
            "the session holds no context {context}; it holds: {}",
            list(&contexts)
        );
        return Err(Error::invalid(why));
    }
    if held.iter().any(|one| one.namespace.is_none()) {
        return Ok(Vec::new());
    }
    let namespaces: Vec<&str> = held
        .iter()
        .filter_map(|one| one.namespace.as_deref())
        .collect();
    let refused = |why: String| {
        Err(Error::invalid(format!(
            "{why}; {context} holds {}",
            list(&namespaces)
        )))
    };
    if read.has("--all-namespaces") {
        return refused(
            "every namespace is read only when the session holds the whole cluster".into(),
        );
    }
    match (read.value("--namespace"), namespaces.as_slice()) {
        (Some(one), _) if namespaces.contains(&one) => Ok(Vec::new()),
        (Some(one), _) => refused(format!("the session does not hold the namespace {one}")),
        (None, [only]) => Ok(vec!["--namespace".to_string(), only.to_string()]),
        (None, _) => refused("name the namespace with -n".into()),
    }
}

/// Whether a get prints the data of a secret it names.
fn shows_secrets(read: &Read) -> bool {
    let printed = read
        .value("--output")
        .is_some_and(|output| !PLAIN.contains(&output));
    printed && read.words.iter().skip(1).any(|word| names_secrets(word))
}

/// `secret`, `secrets/x`, `pods,secrets` or `secrets.v1`.
fn names_secrets(word: &str) -> bool {
    word.split(',').any(|one| {
        let kind = one.split('/').next().unwrap_or_default();
        let base = kind.split('.').next().unwrap_or_default();
        matches!(base.to_ascii_lowercase().as_str(), "secret" | "secrets")
    })
}

/// Whether a log read names how much to read.
fn bounded(read: &Read) -> bool {
    ["--tail", "--since", "--since-time"]
        .iter()
        .any(|one| read.value(one).is_some())
}

fn list(names: &[&str]) -> String {
    match names.is_empty() {
        true => "none".to_string(),
        false => names.join(", "),
    }
}
