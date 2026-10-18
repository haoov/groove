//! The open file's blocks: the guides down their scopes, and the ones folded under their line.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use groove_controllers::AppState;
use groove_controllers::workspace_service::{Block, Touched};

use super::row::edited;
use crate::Ui;
use crate::views::session::Face;

/// The file and the revision the blocks were read for.
type Key = (String, u64);

/// The blocks folded in one file, by the line that opens them, as of buffer revision `seen`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Folds {
    pub seen: u64,
    pub starts: BTreeSet<usize>,
}

/// The blocks last read from the open file's tree, kept while it stays the same.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Blocked {
    kept: RefCell<Option<(Key, Rc<Vec<Block>>)>>,
}

/// The file view's blocks, in line order.
pub(crate) fn of(app: &AppState, ui: &Ui) -> Rc<Vec<Block>> {
    let Some(editor) = edited(app, ui).filter(|_| ui.session.face() == Face::File) else {
        return Rc::default();
    };
    let key = (editor.path.to_string(), editor.buffer.revision());
    let mut kept = ui.blocked.kept.borrow_mut();
    if let Some((at, blocks)) = kept.as_ref()
        && *at == key
    {
        return blocks.clone();
    }
    let blocks = Rc::new(editor.buffer.document().blocks());
    *kept = Some((key, blocks.clone()));
    blocks
}

/// The blocks folded now: those the user folded, but for one the caret stands inside.
pub(crate) fn folded(app: &AppState, ui: &Ui) -> Vec<Block> {
    let Some(editor) = edited(app, ui) else {
        return Vec::new();
    };
    let Some(folds) = ui.session.folds.get(editor.path.as_ref()) else {
        return Vec::new();
    };
    let caret = editor.buffer.caret().line;
    of(app, ui)
        .iter()
        .filter(|one| folds.starts.contains(&one.start))
        .filter(|one| !(one.start < caret && caret <= one.end))
        .copied()
        .collect()
}

/// Whether `line` is one a fold hides.
pub(crate) fn hides(folded: &[Block], line: usize) -> bool {
    folded.iter().any(|one| one.start < line && line <= one.end)
}

/// The guides down `line`: the indent of each block it stands inside.
pub(crate) fn guides(blocks: &[Block], line: usize) -> Vec<usize> {
    let mut columns: Vec<usize> = blocks
        .iter()
        .take_while(|one| one.start < line)
        .filter(|one| line <= one.end)
        .map(|one| one.column)
        .collect();
    columns.dedup();
    columns
}

/// The block `line` opens: whether it is folded, and how many lines it hides.
pub(crate) fn opens(blocks: &[Block], folded: &[Block], line: usize) -> Option<(bool, usize)> {
    let at = blocks.binary_search_by_key(&line, |one| one.start).ok()?;
    let block = blocks[at];
    let shut = folded.iter().any(|one| one.start == line);
    Some((shut, block.end - block.start))
}

/// The open file's folds carried along the lines its edits moved since they were last seen.
pub(crate) fn follow(ui: &mut Ui, app: &AppState) {
    let Some(editor) = edited(app, ui) else {
        return;
    };
    let (path, revision) = (editor.path.to_string(), editor.buffer.revision());
    let Some(folds) = ui
        .session
        .folds
        .get(&path)
        .filter(|one| one.seen != revision)
    else {
        return;
    };
    let blocks = of(app, ui);
    let opens = |line: usize| blocks.binary_search_by_key(&line, |one| one.start).is_ok();
    let starts = match editor.buffer.moves_since(folds.seen) {
        Some(moves) => folds
            .starts
            .iter()
            .filter_map(|start| shifted(*start, &moves, &opens))
            .collect(),
        None => folds.starts.clone(),
    };
    ui.session.folds.insert(
        path,
        Folds {
            seen: revision,
            starts,
        },
    );
}

/// Where a block opening on `start` went; none when its line was replaced.
fn shifted(start: usize, moves: &[Touched], opens: &dyn Fn(usize) -> bool) -> Option<usize> {
    let mut places = vec![start];
    for one in moves {
        let (line, gone, came) = (one.line as usize, one.gone as usize, one.came as usize);
        let mut next = Vec::new();
        for at in places {
            match at {
                at if at >= line + gone => next.push(at + came - gone),
                at if at > line => {}
                at if came > gone => next.extend([at, at + came - gone]),
                at => next.push(at),
            }
        }
        places = next;
    }
    places
        .iter()
        .copied()
        .find(|at| opens(*at))
        .or(places.first().copied())
}
