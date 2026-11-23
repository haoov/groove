//! What a write changes in an object: its YAML before and after, and the unified diff between them.

use imara_diff::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};
use serde_json::Value;

/// The YAML of both sides, as the diff shows an object.
pub(crate) fn sides(before: &Value, after: &Value) -> (String, String) {
    (side(before), side(after))
}

/// One object's YAML, as the diff shows it.
pub(crate) fn side(object: &Value) -> String {
    crate::yaml::yaml(&shown(object))
}

/// The unified diff from `before` to `after`, empty when they are the same.
pub(crate) fn unified(before: &str, after: &str) -> String {
    let input = InternedInput::new(before, after);
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_lines(&input);
    let printer = BasicLineDiffPrinter(&input.interner);
    diff.unified_diff(&printer, UnifiedDiffConfig::default(), &input)
        .to_string()
}

/// The object as the diff shows it: no status, and no field the server alone writes.
fn shown(object: &Value) -> Value {
    let mut object = object.clone();
    if let Some(all) = object.as_object_mut() {
        all.remove("status");
    }
    if let Some(metadata) = object.get_mut("metadata").and_then(Value::as_object_mut) {
        for key in [
            "managedFields",
            "resourceVersion",
            "generation",
            "uid",
            "creationTimestamp",
        ] {
            metadata.remove(key);
        }
    }
    object
}
