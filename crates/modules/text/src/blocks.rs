//! The blocks the syntax tree opens: where each starts, the lines it can fold, and its indent.

use crate::Document;

/// A run of lines a block folds under the line that opens it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// The line that opens it, which stays shown.
    pub start: usize,
    /// The last line it hides.
    pub end: usize,
    /// The display column its opening line's text starts at.
    pub column: usize,
}

impl Document {
    /// The widest block each line opens, in line order.
    pub fn blocks(&self) -> Vec<Block> {
        let Some(syntax) = &self.syntax else {
            return Vec::new();
        };
        let mut spans: Vec<(usize, usize)> = Vec::new();
        let mut cursor = syntax.tree().walk();
        loop {
            let node = cursor.node();
            let (start, last) = (node.start_position().row, node.end_position().row);
            let end = match node.end_position().column {
                0 => last.saturating_sub(1),
                _ => last,
            };
            if node.is_named() && end > start {
                spans.push((start, end));
            }
            if (last > start && cursor.goto_first_child()) || cursor.goto_next_sibling() {
                continue;
            }
            if !climbed(&mut cursor) {
                break;
            }
        }
        spans.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        spans.dedup();
        let shape = self.shape();
        let mut blocks: Vec<Block> = Vec::new();
        for (start, end) in spans {
            if blocks.last().is_some_and(|last| last.start == start) {
                continue;
            }
            blocks.extend(block(&shape, start, end));
        }
        blocks
    }

    /// Each line's indent in display columns, `None` for a blank one, and whether it closes a bracket.
    fn shape(&self) -> Vec<(Option<usize>, bool)> {
        let width = self.indent().width();
        let lines = self.text.lines().take(self.lines());
        lines
            .map(|line| match crate::tabs::indent_of(line.chars(), width) {
                Some((column, first)) => (Some(column), matches!(first, '}' | ')' | ']')),
                None => (None, false),
            })
            .collect()
    }
}

/// The block from `start` to `end`, when what it hides is indented past `start`.
fn block(shape: &[(Option<usize>, bool)], start: usize, end: usize) -> Option<Block> {
    let column = shape.get(start)?.0?;
    let end = match shape.get(end)?.1 {
        true => end.checked_sub(1)?,
        false => end,
    };
    let inside = shape.get(start + 1..=end)?;
    let deeper = inside.iter().all(|(at, _)| at.is_none_or(|at| at > column));
    (end > start && deeper).then_some(Block { start, end, column })
}

/// Up to the next node after the subtree just walked; false at the root.
fn climbed(cursor: &mut tree_sitter::TreeCursor) -> bool {
    while cursor.goto_parent() {
        if cursor.goto_next_sibling() {
            return true;
        }
    }
    false
}
