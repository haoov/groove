//! A list of names, which the file may also give as one name alone.

use serde::Deserialize;

pub(crate) fn one_or_many<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Names {
        One(String),
        Many(Vec<String>),
    }
    Ok(match Names::deserialize(deserializer)? {
        Names::One(name) => vec![name],
        Names::Many(names) => names,
    })
}
