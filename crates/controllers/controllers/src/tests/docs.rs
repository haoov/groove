//! The docs hold the rules and the code holds what exists; these keep the two from drifting.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// Every Markdown file under `docs/`, as `(path, text)`.
fn docs() -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    markdown_under(&repo_root().join("docs"), &mut out);
    assert!(out.len() >= 5, "found {} docs", out.len());
    out
}

fn markdown_under(dir: &Path, out: &mut Vec<(PathBuf, String)>) {
    for entry in fs::read_dir(dir).expect("a directory").flatten() {
        let path = entry.path();
        if path.is_dir() {
            markdown_under(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            let text = fs::read_to_string(&path).expect("a doc");
            out.push((path, text));
        }
    }
}

/// The lines outside fenced code blocks.
fn prose(text: &str) -> impl Iterator<Item = &str> {
    let mut fenced = false;
    text.lines().filter(move |line| {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            return false;
        }
        !fenced
    })
}

/// The text of every `` `span` `` on a line.
fn spans(line: &str) -> impl Iterator<Item = &str> {
    line.split('`').skip(1).step_by(2)
}

/// The target of every `[text](target)` on a line.
fn links(line: &str) -> Vec<&str> {
    line.match_indices("](")
        .filter_map(|(at, _)| {
            let rest = &line[at + 2..];
            rest.find(')').map(|end| &rest[..end])
        })
        .collect()
}

/// The anchor GitHub gives a heading.
fn slug(heading: &str) -> String {
    heading
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

fn anchors(text: &str) -> Vec<String> {
    prose(text)
        .filter(|line| line.starts_with('#'))
        .map(|line| slug(line.trim_start_matches('#')))
        .collect()
}

/// What is wrong with one link from `from`, if anything.
fn broken(from: &Path, own: &str, target: &str) -> Option<String> {
    if target.contains("://") || target.starts_with("mailto:") {
        return None;
    }
    let (file, anchor) = target.split_once('#').unwrap_or((target, ""));
    let path = from.parent().expect("a doc's directory").join(file);
    let text = match file.is_empty() {
        true => own.to_string(),
        false if !path.exists() => return Some(format!("{target}: no such file")),
        false if anchor.is_empty() || path.is_dir() => return None,
        false => fs::read_to_string(&path).expect("a linked doc"),
    };
    let found = anchor.is_empty() || anchors(&text).iter().any(|a| a == anchor);
    (!found).then(|| format!("{target}: no such heading"))
}

#[test]
fn every_link_between_docs_resolves() {
    let mut wrong = Vec::new();
    for (path, text) in docs() {
        for line in prose(&text) {
            for target in links(line) {
                if let Some(why) = broken(&path, &text, target) {
                    wrong.push(format!("{}: {why}", path.display()));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "fix these links:\n{}", wrong.join("\n"));
}

#[test]
fn every_path_a_doc_cites_exists() {
    let root = repo_root();
    let mut gone = Vec::new();
    for (path, text) in docs() {
        for cited in prose(&text)
            .flat_map(spans)
            .filter(|s| s.starts_with("crates/") && !s.contains('<'))
        {
            if !root.join(cited).exists() {
                gone.push(format!("{}: {cited}", path.display()));
            }
        }
    }
    assert!(gone.is_empty(), "these paths moved:\n{}", gone.join("\n"));
}

/// Every crate's directory under `crates/<layer>/`.
fn crate_dirs() -> Vec<PathBuf> {
    let layers = fs::read_dir(repo_root().join("crates")).expect("crates/");
    let dirs: Vec<PathBuf> = layers
        .flatten()
        .filter_map(|layer| fs::read_dir(layer.path()).ok())
        .flat_map(|dir| dir.flatten().map(|entry| entry.path()))
        .filter(|dir| dir.join("Cargo.toml").exists())
        .collect();
    assert!(dirs.len() > 10, "found {} crates", dirs.len());
    dirs
}

/// Whether a crate says what it holds, in its manifest and atop its root file.
fn described(dir: &Path) -> bool {
    let manifest = fs::read_to_string(dir.join("Cargo.toml")).expect("a manifest");
    let said = manifest
        .lines()
        .find_map(|l| l.strip_prefix("description = "))
        .is_some_and(|d| d.trim_matches('"').len() > 10);
    let root = ["src/lib.rs", "src/main.rs"].map(|f| dir.join(f));
    let documented = root
        .iter()
        .find_map(|f| fs::read_to_string(f).ok())
        .is_some_and(|text| text.starts_with("//!"));
    said && documented
}

#[test]
fn every_crate_says_what_it_holds() {
    let silent: Vec<String> = crate_dirs()
        .into_iter()
        .filter(|dir| !described(dir))
        .map(|dir| dir.display().to_string())
        .collect();
    assert!(
        silent.is_empty(),
        "give these a description and a //! doc:\n{}",
        silent.join("\n")
    );
}

/// Every `fn name(` declared under `crates/`.
fn functions() -> Vec<String> {
    let mut out = Vec::new();
    rust_under(&repo_root().join("crates"), &mut out);
    out
}

fn rust_under(dir: &Path, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).expect("a directory").flatten() {
        let path = entry.path();
        if path.is_dir() && path.file_name().is_some_and(|n| n != "target") {
            rust_under(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).expect("a source file");
            out.extend(text.match_indices("fn ").filter_map(|(at, _)| {
                let rest = &text[at + 3..];
                rest.find('(').map(|end| rest[..end].trim().to_string())
            }));
        }
    }
}

#[test]
fn every_test_a_rule_names_exists() {
    let known = functions();
    let mut missing = Vec::new();
    for (path, text) in docs() {
        let named = prose(&text)
            .filter_map(|line| line.split_once("Enforced by").map(|(_, rest)| rest))
            .flat_map(spans);
        for name in named.filter(|n| !known.iter().any(|k| k == n)) {
            missing.push(format!("{}: {name}", path.display()));
        }
    }
    assert!(missing.is_empty(), "no such test:\n{}", missing.join("\n"));
}
