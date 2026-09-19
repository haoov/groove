use std::sync::OnceLock;

use groove_types::{Capture, Indent};
use tree_sitter::{Language as Grammar, Query};

/// A language Groove colours. Everything else is a plain document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Yaml,
    Bash,
    Markdown,
    Python,
    Go,
}

/// The capture names Groove reads, and what each one means. Several names can mean
/// the same thing: grammars spell these differently and change their spelling.
const RECOGNIZED: [(&str, Capture); 20] = [
    ("keyword", Capture::Keyword),
    ("function", Capture::Function),
    ("type", Capture::Type),
    ("constructor", Capture::Type),
    ("string", Capture::String),
    ("character", Capture::String),
    ("number", Capture::Number),
    ("float", Capture::Number),
    ("comment", Capture::Comment),
    ("constant", Capture::Constant),
    ("boolean", Capture::Constant),
    ("attribute", Capture::Attribute),
    ("property", Capture::Attribute),
    ("label", Capture::Attribute),
    ("text.title", Capture::Title),
    ("markup.heading", Capture::Title),
    ("text.literal", Capture::Literal),
    ("markup.raw", Capture::Literal),
    ("text.uri", Capture::Link),
    ("markup.link", Capture::Link),
];

/// What a capture means to us, by the name the grammar gave it.
pub(crate) fn capture(name: &str) -> Option<Capture> {
    RECOGNIZED
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, capture)| *capture)
}

impl Language {
    pub const ALL: [Language; 6] = [
        Language::Rust,
        Language::Yaml,
        Language::Bash,
        Language::Markdown,
        Language::Python,
        Language::Go,
    ];

    /// What one indent step is in this language: what `gofmt`, `rustfmt` and the
    /// like write, so a file keeps the shape its own tools give it.
    pub fn indent(self) -> Indent {
        match self {
            Language::Go => Indent::Tab(4),
            Language::Yaml | Language::Markdown => Indent::Spaces(2),
            Language::Rust | Language::Bash | Language::Python => Indent::Spaces(4),
        }
    }

    /// The language a file name carries, by extension and then by whole name.
    pub fn of(path: &str) -> Option<Language> {
        let name = path.rsplit('/').next().unwrap_or(path);
        let extension = name.rsplit_once('.').map(|(_, end)| end).unwrap_or("");
        let by_extension = match extension.to_ascii_lowercase().as_str() {
            "rs" => Some(Language::Rust),
            "yaml" | "yml" => Some(Language::Yaml),
            "sh" | "bash" | "zsh" => Some(Language::Bash),
            "md" | "markdown" => Some(Language::Markdown),
            "py" | "pyi" => Some(Language::Python),
            "go" => Some(Language::Go),
            _ => None,
        };
        by_extension.or(match name {
            ".bashrc" | ".zshrc" | ".profile" | ".bash_profile" => Some(Language::Bash),
            _ => None,
        })
    }

    fn grammar(self) -> Grammar {
        let language = match self {
            Language::Rust => tree_sitter_rust::LANGUAGE,
            Language::Yaml => tree_sitter_yaml::LANGUAGE,
            Language::Bash => tree_sitter_bash::LANGUAGE,
            Language::Markdown => tree_sitter_md::LANGUAGE,
            Language::Python => tree_sitter_python::LANGUAGE,
            Language::Go => tree_sitter_go::LANGUAGE,
        };
        Grammar::new(language)
    }

    /// Markdown colours its block structure; its inline grammar is a separate parser.
    fn query(self) -> &'static str {
        match self {
            Language::Rust => tree_sitter_rust::HIGHLIGHTS_QUERY,
            Language::Yaml => tree_sitter_yaml::HIGHLIGHTS_QUERY,
            Language::Bash => tree_sitter_bash::HIGHLIGHT_QUERY,
            Language::Markdown => tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
            Language::Python => tree_sitter_python::HIGHLIGHTS_QUERY,
            Language::Go => tree_sitter_go::HIGHLIGHTS_QUERY,
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|it| *it == self).unwrap_or(0)
    }

    /// The query compiled once per language, for every document to share.
    /// The grammar and its highlight query, compiled once for the whole run.
    pub(crate) fn syntax(self) -> Option<&'static (Grammar, Query)> {
        static COMPILED: [OnceLock<Option<(Grammar, Query)>>; Language::ALL.len()] =
            [const { OnceLock::new() }; Language::ALL.len()];
        COMPILED[self.index()]
            .get_or_init(|| compile(self))
            .as_ref()
    }
}

fn compile(language: Language) -> Option<(Grammar, Query)> {
    let grammar = language.grammar();
    let query = Query::new(&grammar, language.query()).ok()?;
    Some((grammar, query))
}
