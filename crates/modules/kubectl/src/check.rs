//! A read checked: its verb, its context and namespace held by the session, no secret's data.

use groove_types::{Attached, Error};

use crate::flags::{Read, read};
use crate::secrets::shows_secrets;

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
    if let Some(why) = read.refused.clone() {
        return Err(Error::invalid(why));
    }
    if read.value("--output").is_some_and(|output| {
        output
            .split('=')
            .next()
            .is_some_and(|format| format.ends_with("-file"))
    }) {
        return Err(Error::invalid(
            "a read names no file: give the template inline, as -o jsonpath=…",
        ));
    }
    let mut argv = vec!["--context".to_string(), context.to_string()];
    argv.extend(args.iter().cloned());
    if !UNSCOPED.contains(&verb.as_str()) {
        argv.extend(scoped(context, &read, attached)?);
    }
    if verb == "get" && shows_secrets(&read) {
        let why = "a secret's data is not read: get secrets as a table, describe one, or name its \
                   metadata fields with -o jsonpath";
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
            let why = format!(
                "{} is not a read; the reads are {}",
                asked(words),
                listed.join(", ")
            );
            Err(Error::invalid(why))
        }
    }
}

/// The words the read was asked by: two for a verb that takes two, as `rollout restart`.
fn asked(words: &[String]) -> String {
    let Some(first) = words.first() else {
        return "nothing".to_string();
    };
    let paired = VERBS.iter().any(|one| one.len() == 2 && one[0] == first);
    match (paired, words.get(1)) {
        (true, Some(second)) => format!("{first} {second}"),
        _ => first.clone(),
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
