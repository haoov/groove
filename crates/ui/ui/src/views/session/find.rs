//! Finding text in what the surface shows: the open file, or the whole change.

use std::ops::Range;

use groove_controllers::AppState;
use groove_controllers::workspace_service::At;
use groove_types::DiffView;

/// One match: where it sits on the surface, and the line it belongs to.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: String,
    /// The row of the whole surface, for the scroll to stand on.
    pub row: usize,
    /// The line of the new side, when the row shows one.
    pub line: Option<usize>,
    pub range: Range<usize>,
}

/// The bar over the rows: what is typed, and what it found.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Finding {
    pub query: crate::widget::Field,
    /// The keyboard is in the bar; on `Enter` it goes back to the code.
    pub typing: bool,
    pub hits: Vec<Hit>,
    pub at: usize,
    /// The view the hits were read from.
    pub view: DiffView,
}

impl Finding {
    pub fn open(view: DiffView) -> Self {
        Self {
            typing: true,
            view,
            ..Self::default()
        }
    }

    pub fn here(&self) -> Option<&Hit> {
        self.hits.get(self.at)
    }

    /// The next match along, or the first one again at the end.
    pub fn step(&mut self, on: bool) {
        if self.hits.is_empty() {
            return;
        }
        let last = self.hits.len() - 1;
        self.at = match (on, self.at) {
            (true, at) if at >= last => 0,
            (true, at) => at + 1,
            (false, 0) => last,
            (false, at) => at - 1,
        };
    }

    /// What the bar says on its right.
    pub fn count(&self) -> String {
        match self.hits.is_empty() {
            true if self.query.is_empty() => String::new(),
            true => "none".to_string(),
            false => format!("{} / {}", self.at + 1, self.hits.len()),
        }
    }

    /// The matches on one row, in the columns of its text.
    pub fn on(&self, row: usize) -> Vec<Range<usize>> {
        self.hits
            .iter()
            .filter(|hit| hit.row == row)
            .map(|hit| hit.range.clone())
            .collect()
    }

    /// The match the bar stands on, when this row is the one it sits on.
    pub fn standing(&self, row: usize) -> Option<Range<usize>> {
        self.here()
            .filter(|hit| hit.row == row)
            .map(|hit| hit.range.clone())
    }
}

/// Every match of `query` in what the view shows, in reading order.
pub fn found(app: &AppState, view: DiffView, query: &str) -> Vec<Hit> {
    if query.is_empty() {
        return Vec::new();
    }
    match view {
        DiffView::Editor => in_file(app, query),
        _ => in_change(app, query),
    }
}

/// The open file, whose rows are its own lines.
fn in_file(app: &AppState, query: &str) -> Vec<Hit> {
    let Some(open) = app.workspace.opened.as_ref() else {
        return Vec::new();
    };
    open.new
        .document()
        .search(query)
        .into_iter()
        .map(|found| Hit {
            path: open.path.clone(),
            row: found.line,
            line: Some(found.line),
            range: found.range,
        })
        .collect()
}

/// Every changed file, read from the rows the surface draws.
fn in_change(app: &AppState, query: &str) -> Vec<Hit> {
    let changes = &app.workspace.changes;
    (0..changes.rows())
        .filter_map(|row| match changes.at(row) {
            Some(At::Row(file, at)) => Some((row, file, at)),
            _ => None,
        })
        .flat_map(|(row, file, at)| {
            let text = file.lines[at].clone();
            let line = file.rows[at].new.map(|line| line as usize);
            let path = file.path.clone();
            matches(&text, query)
                .into_iter()
                .map(move |range| Hit {
                    path: path.clone(),
                    row,
                    line,
                    range,
                })
                .collect::<Vec<Hit>>()
        })
        .collect()
}

/// Where `query` sits in `text`, ignoring case, in characters.
fn matches(text: &str, query: &str) -> Vec<Range<usize>> {
    let (hay, needle) = (text.to_lowercase(), query.to_lowercase());
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = hay[from..].find(&needle) {
        let start = hay[..from + at].chars().count();
        found.push(start..start + needle.chars().count());
        from += at + needle.len();
    }
    found
}
