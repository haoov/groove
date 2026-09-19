//! The colours a surface asked the grammar for, kept while the text stays put.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use groove_controllers::workspace_service::{Colours, Document};

/// Lines read either side of the window, so a small scroll keeps what it has.
const PAD: usize = 40;

/// What the colours of one file and side were read for.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Held {
    stamp: (u64, u64),
    rows: Range<usize>,
    colours: Rc<Colours>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Painted {
    kept: RefCell<HashMap<(String, bool), Held>>,
}

impl Painted {
    /// The colours of `rows`, read again only when the file or the rows moved.
    pub(crate) fn of(
        &self,
        doc: &Document,
        path: &str,
        old: bool,
        stamp: (u64, u64),
        rows: Range<usize>,
    ) -> Rc<Colours> {
        let key = (path.to_string(), old);
        let mut kept = self.kept.borrow_mut();
        if let Some(held) = kept.get(&key)
            && held.stamp == stamp
            && held.rows.start <= rows.start
            && rows.end <= held.rows.end
        {
            return held.colours.clone();
        }
        let rows = rows.start.saturating_sub(PAD)..rows.end + PAD;
        let colours = Rc::new(doc.colours(rows.clone()));
        let held = Held {
            stamp,
            rows,
            colours: colours.clone(),
        };
        kept.insert(key, held);
        colours
    }
}
