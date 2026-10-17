//! The language servers Groove starts, by the languages they serve.

use serde_json::{Value, json};

use crate::Config;

/// One server: how to start it, what it serves, the settings it reads.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LanguageServer {
    pub name: String,
    /// The language ids it serves, as `python` or `yaml`.
    pub languages: Vec<String>,
    pub program: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Variables it does not inherit from Groove, as `VIRTUAL_ENV`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unset: Vec<String>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub settings: Value,
    /// Directories under the worktree it is told about when they move, `*` for any one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub watch: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub off: bool,
}

impl LanguageServer {
    /// ty for Python, and yaml-language-server with the SchemaStore schemas.
    pub fn defaults() -> Vec<LanguageServer> {
        let yaml = json!({ "yaml": {
            "validate": true,
            "hover": true,
            "completion": true,
            "schemaStore": { "enable": true },
        } });
        let ty = Self::new(
            "ty",
            "python",
            "ty",
            &["server"],
            &["VIRTUAL_ENV"],
            Value::Null,
        );
        vec![
            Self {
                watch: vec![".venv/lib/*/site-packages".into()],
                ..ty
            },
            Self::new(
                "yaml-language-server",
                "yaml",
                "yaml-language-server",
                &["--stdio"],
                &[],
                yaml,
            ),
        ]
    }

    fn new(
        name: &str,
        language: &str,
        program: &str,
        args: &[&str],
        unset: &[&str],
        settings: Value,
    ) -> Self {
        let owned = |all: &[&str]| all.iter().map(|one| one.to_string()).collect();
        Self {
            name: name.into(),
            languages: vec![language.into()],
            program: program.into(),
            args: owned(args),
            unset: owned(unset),
            settings,
            watch: Vec::new(),
            off: false,
        }
    }

    pub fn serves(&self, language: &str) -> bool {
        !self.off && self.languages.iter().any(|one| one == language)
    }
}

impl Config {
    /// The servers the file names, or the defaults when it names none.
    pub fn servers(&self) -> Vec<LanguageServer> {
        self.language_servers
            .clone()
            .unwrap_or_else(LanguageServer::defaults)
    }
}
