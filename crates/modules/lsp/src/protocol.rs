//! The messages Groove sends, and what it reads from the answers.

use std::path::{Path, PathBuf};

use groove_types::{Diagnostic, Location, Severity};
use serde_json::{Value, json};

use crate::documents::Documents;
use crate::uri;

pub(crate) fn initialize(root: &Path) -> Value {
    let folder = uri::of(root);
    let name = root.file_name().map(|n| n.to_string_lossy().into_owned());
    json!({
        "processId": std::process::id(),
        "clientInfo": { "name": "groove" },
        "rootUri": folder,
        "workspaceFolders": [{ "uri": folder, "name": name.unwrap_or_default() }],
        "capabilities": {
            "general": { "positionEncodings": ["utf-32", "utf-16"] },
            "workspace": {
                "configuration": true,
                "workspaceFolders": true,
                "didChangeWatchedFiles": { "dynamicRegistration": false },
            },
            "textDocument": {
                "synchronization": { "didSave": true },
                "publishDiagnostics": {},
                "definition": { "linkSupport": true },
                "hover": { "contentFormat": ["markdown", "plaintext"] },
            },
        },
    })
}

/// The path a `publishDiagnostics` names, and its diagnostics in characters.
pub(crate) fn diagnostics(
    params: &Value,
    documents: &Documents,
) -> Option<(PathBuf, Vec<Diagnostic>)> {
    let path = uri::path(params.get("uri")?.as_str()?)?;
    let found = params.get("diagnostics")?.as_array()?;
    let read = found
        .iter()
        .filter_map(|d| diagnostic(d, &path, documents))
        .collect();
    Some((path, read))
}

fn diagnostic(found: &Value, path: &Path, documents: &Documents) -> Option<Diagnostic> {
    let range = found.get("range")?;
    let severity = match found.get("severity").and_then(Value::as_u64) {
        Some(2) => Severity::Warning,
        Some(3) => Severity::Information,
        Some(4) => Severity::Hint,
        _ => Severity::Error,
    };
    let text = |key: &str| match found.get(key)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    };
    Some(Diagnostic {
        start: documents.caret(path, range.get("start")?),
        end: documents.caret(path, range.get("end")?),
        severity,
        message: found.get("message")?.as_str()?.to_string(),
        source: text("source"),
        code: text("code"),
    })
}

/// A definition's answer: one location, a list of them, or a list of links.
pub(crate) fn locations(answer: &Value, documents: &Documents) -> Vec<Location> {
    let one = |found: &Value| {
        let (target, range) = match found.get("targetUri") {
            Some(target) => (target, found.get("targetSelectionRange")?),
            None => (found.get("uri")?, found.get("range")?),
        };
        let path = uri::path(target.as_str()?)?;
        Some(Location {
            start: documents.caret(&path, range.get("start")?),
            end: documents.caret(&path, range.get("end")?),
            path,
        })
    };
    match answer {
        Value::Array(found) => found.iter().filter_map(one).collect(),
        Value::Object(_) => one(answer).into_iter().collect(),
        _ => Vec::new(),
    }
}

/// A hover's contents as one text; `None` when it says nothing.
pub(crate) fn hover(answer: &Value) -> Option<String> {
    let text = said(answer.get("contents")?);
    (!text.trim().is_empty()).then_some(text)
}

fn said(contents: &Value) -> String {
    match contents {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts.iter().map(said).collect::<Vec<_>>().join("\n\n"),
        Value::Object(part) => {
            let value = part
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or_default();
            match part.get("language").and_then(Value::as_str) {
                Some(language) => format!("```{language}\n{value}\n```"),
                None => value.to_string(),
            }
        }
        _ => String::new(),
    }
}

/// The settings under a dotted `section`, all of them when it names none, null when absent.
pub(crate) fn section(settings: &Value, section: Option<&str>) -> Value {
    let Some(section) = section.filter(|s| !s.is_empty()) else {
        return settings.clone();
    };
    section
        .split('.')
        .try_fold(settings, |at, key| at.get(key))
        .cloned()
        .unwrap_or(Value::Null)
}
