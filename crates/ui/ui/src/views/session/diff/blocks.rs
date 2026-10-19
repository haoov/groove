//! The open file's blocks: the guides down their scopes, and the ones folded under their line.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use groove_controllers::AppState;
pub(crate) use groove_controllers::workspace_service::Block;
use groove_controllers::workspace_service::Touched;

use super::row::edited;
use crate::Ui;
use crate::views::session::Face;

/// The file and the revision the blocks were read for.
type Key = (String, u64);

/// The blocks, the caret's line and the folds the folded blocks were picked for.
type Picked = (Arc<Vec<Block>>, usize, Folds);

/// The blocks folded in one file, by the line that opens them, as of buffer revision `seen`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Folds {
    pub seen: u64,
    pub starts: BTreeSet<usize>,
}

/// The blocks last read on this thread, and the folded ones last picked, kept while they hold.
#[derive(Debug, Default, Clone)]
pub struct Blocked {
    kept: RefCell<Option<(Key, Arc<Vec<Block>>)>>,
    picked: RefCell<Option<(Picked, Rc<Vec<Block>>)>>,
}

impl PartialEq for Blocked {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// The file view's blocks, in line order: the parse job's, else read here once a revision.
pub(crate) fn of(app: &AppState, ui: &Ui) -> Arc<Vec<Block>> {
    let Some(editor) = edited(app, ui).filter(|_| ui.session.face() == Face::File) else {
        return Arc::default();
    };
    if let Some(blocks) = editor.buffer.blocks() {
        return blocks;
    }
    let key = (editor.path.to_string(), editor.buffer.revision());
    let mut kept = ui.blocked.kept.borrow_mut();
    if let Some((at, blocks)) = kept.as_ref()
        && *at == key
    {
        return blocks.clone();
    }
    let blocks = Arc::new(editor.buffer.document().blocks());
    *kept = Some((key, blocks.clone()));
    blocks
}

/// The blocks folded now, in line order: those the user folded, but for one the caret stands inside.
pub(crate) fn folded(app: &AppState, ui: &Ui) -> Rc<Vec<Block>> {
    let Some(editor) = edited(app, ui) else {
        return Rc::default();
    };
    let Some(folds) = ui.session.folds.get(editor.path.as_ref()) else {
        return Rc::default();
    };
    let blocks = of(app, ui);
    let caret = editor.buffer.caret().line;
    let mut picked = ui.blocked.picked.borrow_mut();
    if let Some(((read, line, held), folded)) = picked.as_ref()
        && Arc::ptr_eq(read, &blocks)
        && (*line, held) == (caret, folds)
    {
        return folded.clone();
    }
    let folded: Rc<Vec<Block>> = Rc::new(
        blocks
            .iter()
            .filter(|one| folds.starts.contains(&one.start))
            .filter(|one| !(one.start < caret && caret <= one.end))
            .copied()
            .collect(),
    );
    *picked = Some(((blocks.clone(), caret, folds.clone()), folded.clone()));
    folded
}

/// Whether each of `lines` lines is one a fold hides, `folded` in line order.
pub(crate) fn hidden(folded: &[Block], lines: usize) -> Vec<bool> {
    let mut out = vec![false; lines];
    let mut until = 0;
    for one in folded {
        let from = (one.start + 1).max(until);
        let to = (one.end + 1).min(lines);
        out.iter_mut()
            .take(to)
            .skip(from)
            .for_each(|line| *line = true);
        until = until.max(to);
    }
    out
}

/// The guides down each of `lines`: the indent of each block it stands inside.
pub(crate) fn guides(blocks: &[Block], lines: Range<usize>) -> Vec<Vec<usize>> {
    let mut out = vec![Vec::new(); lines.len()];
    let near = blocks.iter().take_while(|one| one.start + 1 < lines.end);
    for one in near.filter(|one| one.end >= lines.start) {
        let (from, to) = (
            (one.start + 1).max(lines.start),
            (one.end + 1).min(lines.end),
        );
        for columns in &mut out[from - lines.start..to - lines.start] {
            if columns.last() != Some(&one.column) {
                columns.push(one.column);
            }
        }
    }
    out
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
