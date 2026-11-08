//! What a write changes in an object: its YAML before and after, and the unified diff between them.

use imara_diff::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};
use serde_json::Value;

/// The YAML of both sides, without the fields every write moves and `managedFields`.
pub(crate) fn sides(before: &Value, after: &Value) -> (String, String) {
    (
        crate::yaml::yaml(&shown(before)),
        crate::yaml::yaml(&shown(after)),
    )
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

fn shown(object: &Value) -> Value {
    let mut object = object.clone();
    if let Some(metadata) = object.get_mut("metadata").and_then(Value::as_object_mut) {
        metadata.remove("resourceVersion");
        metadata.remove("generation");
        metadata.remove("managedFields");
    }
    object
}
