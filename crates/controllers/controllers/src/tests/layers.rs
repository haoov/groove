//! Every crate's `groove-*` dependencies must point down the layer table. The `groove`
//! binary is the wiring and may name any layer.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const LAYERS: [&str; 6] = ["base", "shared", "modules", "services", "controllers", "ui"];

/// The layers a crate of the given layer may depend on.
fn allowed(layer: &str) -> &'static [&'static str] {
    match layer {
        "base" | "shared" => &[],
        "modules" => &["base", "shared", "modules"],
        "services" => &["modules", "shared"],
        "controllers" => &["services", "shared"],
        "ui" => &["controllers", "shared", "base", "ui"],
        other => panic!("unknown layer {other}"),
    }
}

fn crates_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `(package name, layer, groove dependencies)` per crate under `crates/`.
fn crates() -> Vec<(String, String, Vec<String>)> {
    let mut out = Vec::new();
    for layer in LAYERS {
        let Ok(dir) = fs::read_dir(crates_root().join(layer)) else {
            continue;
        };
        for entry in dir.flatten() {
            let manifest = entry.path().join("Cargo.toml");
            let Ok(text) = fs::read_to_string(&manifest) else {
                continue;
            };
            out.push((package_name(&text), layer.to_string(), groove_deps(&text)));
        }
    }
    out
}

fn package_name(manifest: &str) -> String {
    manifest
        .lines()
        .find_map(|l| l.strip_prefix("name = "))
        .map(|v| v.trim_matches('"').to_string())
        .unwrap_or_default()
}

fn groove_deps(manifest: &str) -> Vec<String> {
    let mut in_deps = false;
    let mut deps = Vec::new();
    for line in manifest.lines() {
        if line.starts_with('[') {
            in_deps = line.starts_with("[dependencies]") || line.starts_with("[dev-dependencies]");
            continue;
        }
        if in_deps && line.starts_with("groove-") {
            deps.push(
                line.split(['=', '.', ' '])
                    .next()
                    .unwrap_or_default()
                    .to_string(),
            );
        }
    }
    deps
}

#[test]
fn every_dependency_points_down_the_layers() {
    let crates = crates();
    assert!(crates.len() >= 6, "found {} crates", crates.len());
    let layer_of: HashMap<&str, &str> = crates
        .iter()
        .map(|(n, l, _)| (n.as_str(), l.as_str()))
        .collect();
    let mut offenders = Vec::new();
    for (name, layer, deps) in &crates {
        if name == "groove" {
            continue;
        }
        for dep in deps {
            let dep_layer = layer_of
                .get(dep.as_str())
                .unwrap_or_else(|| panic!("{name}: unknown crate {dep}"));
            if !allowed(layer).contains(dep_layer) {
                offenders.push(format!("{name} ({layer}) -> {dep} ({dep_layer})"));
            }
        }
    }
    assert!(offenders.is_empty(), "{offenders:#?}");
}

#[test]
fn no_service_depends_on_a_service() {
    let crates = crates();
    let services: Vec<&str> = crates
        .iter()
        .filter(|(_, l, _)| l == "services")
        .map(|(n, _, _)| n.as_str())
        .collect();
    for (name, layer, deps) in &crates {
        if layer == "services" {
            assert!(
                deps.iter().all(|d| !services.contains(&d.as_str())),
                "{name} depends on a service"
            );
        }
    }
}

/// The ceilings the architecture sets: a file, a function, and a test's own two.
const FILE_MAX: usize = 300;
const FN_MAX: usize = 50;
const TEST_FILE_MAX: usize = 500;
const TEST_FN_MAX: usize = 100;

/// Every `.rs` file under `crates/`, as `(path from the root, text)`.
fn sources() -> Vec<(String, String)> {
    let root = crates_root();
    let mut out = Vec::new();
    walk(&root, &root, &mut out);
    out.sort();
    assert!(out.len() > 100, "found {} files", out.len());
    out
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).expect("a directory").flatten() {
        let path = entry.path();
        if path.is_dir() && path.file_name().is_some_and(|name| name != "examples") {
            walk(&path, root, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let at = path.strip_prefix(root).expect("under crates");
            let text = fs::read_to_string(&path).expect("a source file");
            out.push((at.to_string_lossy().into_owned(), text));
        }
    }
}

/// Whether a file holds tests rather than the code they test.
fn is_test(path: &str) -> bool {
    path.contains("tests/") || path.ends_with("tests.rs")
}

#[test]
fn no_file_stands_above_its_ceiling() {
    let long: Vec<String> = sources()
        .iter()
        .map(|(path, text)| (path, text.lines().count()))
        .filter(|(path, lines)| match is_test(path) {
            true => *lines > TEST_FILE_MAX,
            false => *lines > FILE_MAX,
        })
        .map(|(path, lines)| format!("{path}: {lines} lines"))
        .collect();
    assert!(long.is_empty(), "split these:\n{}", long.join("\n"));
}

#[test]
fn no_function_stands_above_its_ceiling() {
    let mut long = Vec::new();
    for (path, text) in sources() {
        let lines: Vec<&str> = text.lines().collect();
        let mut open: Option<(usize, usize, String)> = None;
        for (at, line) in lines.iter().enumerate() {
            if let Some((start, indent, name)) = &open
                && *line == format!("{}}}", " ".repeat(*indent))
            {
                let count = at - start + 1;
                let most = match is_test(&path) {
                    true => TEST_FN_MAX,
                    false => FN_MAX,
                };
                if count > most {
                    long.push(format!("{path}:{} {name}: {count} lines", start + 1));
                }
                open = None;
                continue;
            }
            if open.is_none()
                && let Some(name) = declares(line)
            {
                let indent = line.len() - line.trim_start().len();
                open = Some((at, indent, name));
            }
        }
    }
    assert!(long.is_empty(), "split these:\n{}", long.join("\n"));
}

/// The name a line declares a function with, when it opens one.
fn declares(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("pub(crate) ")
        .or_else(|| trimmed.strip_prefix("pub(super) "))
        .or_else(|| trimmed.strip_prefix("pub "))
        .unwrap_or(trimmed);
    let rest = rest.strip_prefix("async ").unwrap_or(rest);
    let rest = rest.strip_prefix("fn ")?;
    if line.trim_end().ends_with(';') {
        return None;
    }
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}
