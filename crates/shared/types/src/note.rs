use crate::{AnnotationId, Timestamp};

/// Where a note sits in a file, on the new side.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Anchor {
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
}

impl Anchor {
    /// One line of a file.
    pub fn line(path: impl Into<String>, line: u32) -> Self {
        Anchor {
            path: path.into(),
            start_line: line,
            end_line: line,
        }
    }

    /// Lines numbered from 1, in either order; `None` past what a file can number.
    pub fn numbered(path: impl Into<String>, line: i64, end: Option<i64>) -> Option<Self> {
        let row = |one: i64| u32::try_from(one.max(1) - 1).ok();
        let (start, end) = (row(line)?, row(end.unwrap_or(line))?);
        Some(Anchor {
            path: path.into(),
            start_line: start.min(end),
            end_line: start.max(end),
        })
    }

    pub fn rows(&self) -> std::ops::RangeInclusive<u32> {
        self.start_line..=self.end_line
    }
}

/// Whose note it is: this session's own, or a thread on the merge request.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum NoteOrigin {
    Local(AnnotationId),
    Thread(String),
}

/// One thing said in a note: an annotation says one, a thread says several.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Said {
    pub author: String,
    pub body: String,
    pub at: Timestamp,
}

/// A note to act on; `anchor` is absent on a comment on the MR itself.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Note {
    pub origin: NoteOrigin,
    pub anchor: Option<Anchor>,
    pub resolved: bool,
    pub said: Vec<Said>,
}

impl Note {
    pub fn is_local(&self) -> bool {
        matches!(self.origin, NoteOrigin::Local(_))
    }

    /// The note's own id, for the writes that take one.
    pub fn id(&self) -> Option<&AnnotationId> {
        match &self.origin {
            NoteOrigin::Local(id) => Some(id),
            NoteOrigin::Thread(_) => None,
        }
    }

    /// What the list shows first: the words it opens with.
    pub fn opening(&self) -> Option<&Said> {
        self.said.first()
    }

    pub fn at(&self) -> Option<Timestamp> {
        self.opening().map(|said| said.at)
    }

    /// How many replies stand under the opening words.
    pub fn replies(&self) -> usize {
        self.said.len().saturating_sub(1)
    }

    /// Whether the note stands on any of these rows of this file.
    pub fn over(&self, path: &str, lines: (u32, u32)) -> bool {
        self.anchor.as_ref().is_some_and(|one| {
            one.path == path && one.start_line <= lines.1 && lines.0 <= one.end_line
        })
    }

    /// Whether the note stands on this row of this file.
    pub fn on(&self, path: &str, line: u32) -> bool {
        self.anchor
            .as_ref()
            .is_some_and(|one| one.path == path && one.rows().contains(&line))
    }
}

/// The labels Conventional Comments names.
pub const LABELS: [&str; 12] = [
    "praise",
    "nitpick",
    "suggestion",
    "issue",
    "todo",
    "question",
    "thought",
    "chore",
    "note",
    "typo",
    "polish",
    "quibble",
];

/// A comment's Conventional Comments label and its decorations, when its first line opens on them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Labelled<'a> {
    pub label: &'a str,
    pub decorations: Vec<&'a str>,
    /// What the comment says once its label is taken off.
    pub rest: &'a str,
}

impl Labelled<'_> {
    /// The prefix that writes this label and these decorations again.
    pub fn prefix(label: &str, decorations: &[&str]) -> String {
        match decorations {
            [] => format!("{label}: "),
            many => format!("{label} ({}): ", many.join(", ")),
        }
    }
}

/// `issue (agent, blocking): it leaks`, bold or not, read as its label, decorations and words.
pub fn labelled(body: &str) -> Option<Labelled<'_>> {
    let (head, rest) = body.split_once(':')?;
    let head = head.trim();
    let head = head.strip_prefix("**").unwrap_or(head);
    let head = head.strip_suffix("**").unwrap_or(head);
    let rest = rest.strip_prefix("**").unwrap_or(rest);
    let (label, decorations) = match head.split_once('(') {
        Some((label, inside)) => (label.trim(), inside.strip_suffix(')')?),
        None => (head.trim(), ""),
    };
    let label = LABELS.iter().find(|one| label.eq_ignore_ascii_case(one))?;
    Some(Labelled {
        label,
        decorations: decorations
            .split(',')
            .map(str::trim)
            .filter(|one| !one.is_empty())
            .collect(),
        rest: rest.trim_start(),
    })
}

/// The comment with `decoration` among its label's, its label `note` when it had none.
pub fn decorated(body: &str, decoration: &str) -> String {
    let Some(held) = labelled(body) else {
        return format!("{}{body}", Labelled::prefix("note", &[decoration]));
    };
    if held.decorations.contains(&decoration) {
        return body.to_string();
    }
    let mut decorations = vec![decoration];
    decorations.extend(held.decorations.iter().copied());
    format!(
        "{}{}",
        Labelled::prefix(held.label, &decorations),
        held.rest
    )
}
