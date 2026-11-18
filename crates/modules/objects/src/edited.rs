//! An object's YAML as edited, read back and turned into the merge patch the edit makes.

use groove_types::{Error, Result};
use serde_json::{Map, Number, Value};
use yaml_rust2::{Yaml, YamlLoader};

/// What only the server writes: an edit that changes one is refused.
const SERVER: [&str; 6] = [
    "/status",
    "/metadata/resourceVersion",
    "/metadata/uid",
    "/metadata/generation",
    "/metadata/creationTimestamp",
    "/metadata/managedFields",
];

/// The merge patch from `before` to `after`, both an object's YAML.
pub fn edited(before: &str, after: &str) -> Result<Value> {
    let (before, after) = (parsed(before)?, parsed(after)?);
    let changed = |at: &&&str| before.pointer(at) != after.pointer(at);
    let moved = SERVER.iter().find(changed);
    if let Some(at) = moved {
        let field = at.trim_start_matches('/').replace('/', ".");
        return Err(Error::invalid(format!(
            "{field} is written by the server: put it back as it was"
        )));
    }
    let patch = merge_patch(&before, &after);
    if patch.as_object().is_some_and(Map::is_empty) {
        return Err(Error::invalid("the edit changes nothing"));
    }
    Ok(patch)
}

/// One YAML document, as JSON values.
pub(crate) fn parsed(text: &str) -> Result<Value> {
    let read = YamlLoader::load_from_str(text);
    let documents = read.map_err(|e| Error::invalid(format!("the YAML does not read: {e}")))?;
    match documents.as_slice() {
        [one] => value(one),
        _ => Err(Error::invalid("the YAML must hold one document")),
    }
}

fn value(yaml: &Yaml) -> Result<Value> {
    Ok(match yaml {
        Yaml::Null => Value::Null,
        Yaml::Boolean(one) => Value::Bool(*one),
        Yaml::Integer(one) => Value::from(*one),
        Yaml::Real(text) => real(text)?,
        Yaml::String(one) => Value::String(one.clone()),
        Yaml::Array(items) => Value::Array(items.iter().map(value).collect::<Result<_>>()?),
        Yaml::Hash(pairs) => {
            let mut map = Map::new();
            for (key, one) in pairs {
                let Yaml::String(key) = key else {
                    return Err(Error::invalid("a key must be text"));
                };
                map.insert(key.clone(), value(one)?);
            }
            Value::Object(map)
        }
        Yaml::Alias(_) => return Err(Error::invalid("anchors and aliases are not read")),
        Yaml::BadValue => return Err(Error::invalid("the YAML holds a value it cannot read")),
    })
}

fn real(text: &str) -> Result<Value> {
    let number = text.parse::<f64>().ok().and_then(Number::from_f64);
    let refused = || Error::invalid(format!("{text} is not a number JSON holds"));
    number.map(Value::Number).ok_or_else(refused)
}

/// RFC 7386: what changed from `before` to `after`, a key gone as null.
pub(crate) fn merge_patch(before: &Value, after: &Value) -> Value {
    let (Value::Object(was), Value::Object(now)) = (before, after) else {
        return after.clone();
    };
    let mut patch = Map::new();
    for (key, one) in now {
        match was.get(key) {
            Some(old) if old == one => {}
            Some(old) if old.is_object() && one.is_object() => {
                patch.insert(key.clone(), merge_patch(old, one));
            }
            _ => {
                patch.insert(key.clone(), one.clone());
            }
        }
    }
    for key in was.keys().filter(|key| !now.contains_key(*key)) {
        patch.insert(key.clone(), Value::Null);
    }
    Value::Object(patch)
}
