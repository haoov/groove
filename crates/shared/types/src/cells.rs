//! Two cells of a Table in order: a number or an age by its value, anything else as text.

use std::cmp::Ordering;

/// Two numbers or two ages by value, else as text.
pub fn compare_cells(a: &str, b: &str) -> Ordering {
    match (value(a), value(b)) {
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
        _ => a.cmp(b),
    }
}

/// A leading number, `3 (5m ago)` reads 3; or an age as `kubectl` writes it, `2d4h`, in seconds.
fn value(text: &str) -> Option<f64> {
    let text = text.trim();
    let lead: String = text
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if !lead.is_empty() && !text[lead.len()..].starts_with(|c: char| "smhdy".contains(c)) {
        return lead.parse().ok();
    }
    age(text)
}

/// An age as `kubectl` writes it, in whole seconds.
pub(crate) fn read_age(text: &str) -> Option<u64> {
    age(text.trim()).map(|seconds| seconds as u64)
}

fn age(text: &str) -> Option<f64> {
    let (mut total, mut number) = (0.0, String::new());
    for c in text.chars() {
        match c {
            '0'..='9' => number.push(c),
            's' | 'm' | 'h' | 'd' | 'y' if !number.is_empty() => {
                let unit = match c {
                    's' => 1.0,
                    'm' => 60.0,
                    'h' => 3_600.0,
                    'd' => 86_400.0,
                    _ => 31_536_000.0,
                };
                total += number.parse::<f64>().ok()? * unit;
                number.clear();
            }
            _ => return None,
        }
    }
    number.is_empty().then_some(total)
}
