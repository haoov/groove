//! What a get may print of a secret: its table, its name, or its metadata by named field.

use crate::flags::Read;

/// The outputs of a get that show no secret's data.
const PLAIN: &[&str] = &["wide", "name"];

/// The fields under `.metadata` a secret's jsonpath may name.
const METADATA: &[&str] = &[
    "name",
    "namespace",
    "uid",
    "resourceVersion",
    "generation",
    "creationTimestamp",
    "deletionTimestamp",
    "labels",
    "ownerReferences",
    "finalizers",
    "managedFields",
];

/// Whether a get prints more of a secret it names than its metadata.
pub(crate) fn shows_secrets(read: &Read) -> bool {
    if !read.words.iter().skip(1).any(|word| names_secrets(word)) {
        return false;
    }
    let printed = match read.value("--output") {
        None => false,
        Some(output) if PLAIN.contains(&output) => false,
        Some(output) => !output.strip_prefix("jsonpath=").is_some_and(only_metadata),
    };
    let sorted = read
        .value("--sort-by")
        .is_some_and(|path| !metadata_path(path));
    printed || sorted
}

/// `secret`, `secrets/x`, `pods,secrets` or `secrets.v1`.
fn names_secrets(word: &str) -> bool {
    word.split(',').any(|one| {
        let kind = one.split('/').next().unwrap_or_default();
        let base = kind.split('.').next().unwrap_or_default();
        matches!(base.to_ascii_lowercase().as_str(), "secret" | "secrets")
    })
}

/// Whether every `{…}` of a jsonpath template names a literal, a range of items, or metadata.
fn only_metadata(template: &str) -> bool {
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            return false;
        };
        let one = rest[open + 1..open + close].trim();
        let allowed = match one {
            "end" | "range .items[*]" | "range .items" => true,
            literal if literal.starts_with('"') && literal.ends_with('"') => true,
            path => metadata_path(path),
        };
        if !allowed {
            return false;
        }
        rest = &rest[open + close + 1..];
    }
    true
}

/// `.metadata.name`, `.items[*].metadata.labels`, `.type`, or one annotation by its key.
fn metadata_path(path: &str) -> bool {
    let path = path
        .trim_start_matches(['{', ' '])
        .trim_end_matches(['}', ' ']);
    if path.contains("..") || path.contains('?') {
        return false;
    }
    let path = path.strip_prefix(".items[*]").unwrap_or(path);
    if path == ".type" {
        return true;
    }
    let Some(field) = path.strip_prefix(".metadata.") else {
        return false;
    };
    let head = field.split(['.', '[']).next().unwrap_or_default();
    match head {
        "annotations" => one_annotation(&field["annotations".len()..]),
        named => METADATA.contains(&named),
    }
}

/// `.key` or `['key']` after `annotations`, never the last applied copy of the object.
fn one_annotation(key: &str) -> bool {
    let key = key.trim_start_matches('.');
    let key = key.trim_start_matches("['").trim_end_matches("']");
    !key.is_empty() && !key.contains("last-applied")
}
