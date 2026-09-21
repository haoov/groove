//! The crate's own shape. Numbers live in one file, styles in one file, and a view
//! draws through the context.

use std::fs;
use std::path::Path;

/// Bare numbers any file may write: an origin, a unit, a half.
const ALLOWED: [&str; 3] = ["0.0", "1.0", "2.0"];

/// What a view may take from the renderer: geometry, nothing that draws or paints.
const GEOMETRY: [&str; 2] = ["Rect", "Size"];

/// Every source file of the crate but the tests, as `(path from src, text)`.
fn sources() -> Vec<(String, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    walk(&src, &src, &mut out);
    out.sort();
    assert!(out.len() > 15, "found {} files", out.len());
    out
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).expect("a directory").flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() && name != "tests" {
            walk(&path, root, out);
        } else if path.extension().is_some_and(|e| e == "rs") && name != "tests.rs" {
            let relative = path
                .strip_prefix(root)
                .expect("under src")
                .to_string_lossy()
                .into_owned();
            out.push((relative, fs::read_to_string(&path).expect("a source file")));
        }
    }
}

/// The decimal numbers written out in a line of code.
fn decimals(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        let after_name = start > 0 && {
            let previous = bytes[start - 1];
            previous.is_ascii_alphanumeric() || previous == b'_' || previous == b'.'
        };
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        if after_name {
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'.' && bytes[i + 1].is_ascii_digit() {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            out.push(line[start..i].to_string());
        }
    }
    out
}

fn code_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
}

#[test]
fn every_size_comes_from_the_tokens() {
    let mut offenders = Vec::new();
    for (path, text) in sources() {
        if path == "tokens.rs" {
            continue;
        }
        for (at, line) in code_lines(&text) {
            for number in decimals(line) {
                if !ALLOWED.contains(&number.as_str()) {
                    offenders.push(format!("{path}:{}: {number}", at + 1));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "sizes belong in tokens.rs: {offenders:#?}"
    );
}

#[test]
fn every_style_comes_from_one_file() {
    let mut offenders = Vec::new();
    for (path, text) in sources() {
        if path == "style.rs" {
            continue;
        }
        for (at, line) in code_lines(&text) {
            if line.contains("TextStyle {") {
                offenders.push(format!("{path}:{}", at + 1));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "styles belong in style.rs: {offenders:#?}"
    );
}

#[test]
fn a_view_draws_through_the_context() {
    let mut offenders = Vec::new();
    for (path, text) in sources() {
        if !path.starts_with("views/") {
            continue;
        }
        for (at, line) in code_lines(&text) {
            let Some(rest) = line.split_once("groove_gfx::").map(|(_, rest)| rest) else {
                continue;
            };
            let items: Vec<&str> = rest
                .trim_end_matches(';')
                .trim_matches(|c: char| c == '{' || c == '}')
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .collect();
            if !items.iter().all(|item| GEOMETRY.contains(item)) {
                offenders.push(format!("{path}:{}: {}", at + 1, line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a view takes geometry from the renderer and everything else from Ctx: {offenders:#?}"
    );
}

/// Every ground and band a row can take, in each flavour.
fn grounds(theme: groove_types::ThemeName) -> Vec<(&'static str, groove_gfx::Color)> {
    let styles = crate::style::Styles::new(theme, crate::tokens::Tokens::new(1.0));
    vec![
        ("hover", styles.hover()),
        ("raised", styles.raised()),
        ("action", styles.action()),
        ("held", styles.held()),
        ("noted", styles.noted()),
        ("here", styles.here()),
        ("band", styles.band()),
        ("deep", styles.deep()),
        ("ground", styles.ground()),
    ]
}

#[test]
fn no_two_grounds_a_row_can_take_share_a_value() {
    for theme in [
        groove_types::ThemeName::Latte,
        groove_types::ThemeName::Frappe,
        groove_types::ThemeName::Macchiato,
        groove_types::ThemeName::Mocha,
    ] {
        let grounds = grounds(theme);
        for (at, (name, color)) in grounds.iter().enumerate() {
            let same = grounds
                .iter()
                .skip(at + 1)
                .find(|(_, other)| other == color)
                .map(|(other, _)| *other);
            assert!(
                same.is_none(),
                "{theme:?}: {name} and {} share a value",
                same.unwrap_or("")
            );
        }
    }
}

/// How much brighter one colour is than another, by WCAG's own reckoning.
fn contrast(one: groove_gfx::Color, two: groove_gfx::Color) -> f32 {
    let light = |c: groove_gfx::Color| {
        let part = |v: u8| {
            let v = f32::from(v) / 255.0;
            match v <= 0.04045 {
                true => v / 12.92,
                false => ((v + 0.055) / 1.055).powf(2.4),
            }
        };
        0.2126 * part(c.r) + 0.7152 * part(c.g) + 0.0722 * part(c.b)
    };
    let (one, two) = (light(one), light(two));
    (one.max(two) + 0.05) / (one.min(two) + 0.05)
}

/// The grounds a line of code is drawn on, which its text must stand out from.
const UNDER_CODE: [&str; 6] = ["held", "noted", "hover", "band", "deep", "ground"];

#[test]
fn code_stays_readable_on_every_ground_it_is_drawn_on() {
    for theme in [
        groove_types::ThemeName::Latte,
        groove_types::ThemeName::Frappe,
        groove_types::ThemeName::Macchiato,
        groove_types::ThemeName::Mocha,
    ] {
        let styles = crate::style::Styles::new(theme, crate::tokens::Tokens::new(1.0));
        let text = styles.color(crate::style::Role::Text);
        for (named, ground) in grounds(theme) {
            if !UNDER_CODE.contains(&named) {
                continue;
            }
            let ratio = contrast(text, ground);
            assert!(
                ratio >= 4.5,
                "{theme:?} {named}: code stands at {ratio:.2} to 1 on it"
            );
        }
    }
}
