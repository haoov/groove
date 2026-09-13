//! Every crate's `groove-*` dependencies must point down the layer table.

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
