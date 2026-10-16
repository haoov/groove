//! What the path term left, built again only when the term, the paths or the change move.

use std::cell::RefCell;
use std::rc::Rc;

/// Each row the term kept: a changed file, or a worktree path, by its place in its list.
pub(crate) type Kept = Rc<Vec<(bool, usize)>>;

/// The term, the paths' stamp and the change's stamp the rows were kept for.
type Key = (String, u64, u64);

/// The last rows kept, and what they were kept for.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Narrowing {
    kept: RefCell<Option<(Key, Kept)>>,
}

impl Narrowing {
    pub(crate) fn of(
        &self,
        key: (String, u64, u64),
        build: impl FnOnce() -> Vec<(bool, usize)>,
    ) -> Kept {
        let mut kept = self.kept.borrow_mut();
        if let Some((at, rows)) = kept.as_ref()
            && *at == key
        {
            return rows.clone();
        }
        let rows = Rc::new(build());
        *kept = Some((key, rows.clone()));
        rows
    }
}
