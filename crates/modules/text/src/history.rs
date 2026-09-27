//! The undo history: one change, and the two stacks.

/// One change to a document: what `at` held, and what it holds now.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Change {
    pub(crate) at: usize,
    pub(crate) removed: String,
    pub(crate) inserted: String,
}

impl Change {
    /// The change that puts the document back.
    pub(crate) fn inverse(&self) -> Self {
        Self {
            at: self.at,
            removed: self.inserted.clone(),
            inserted: self.removed.clone(),
        }
    }

    /// Where the caret belongs once this change is applied.
    pub(crate) fn after(&self) -> usize {
        self.at + self.inserted.chars().count()
    }

    /// Whether `next` carries on typing where this change stopped.
    fn continues(&self, next: &Change) -> bool {
        next.removed.is_empty() && !next.inserted.contains('\n') && next.at == self.after()
    }
}

/// One keystroke's changes, one per caret, undone together.
type Step = Vec<Change>;

/// A typing run undone in one step; a motion or a newline closes it.
#[derive(Default, Debug)]
pub(crate) struct History {
    done: Vec<Step>,
    undone: Vec<Step>,
    open: bool,
}

impl History {
    /// Records a step, joining it to the run before it when it carries on typing.
    pub(crate) fn did(&mut self, step: Step) {
        self.undone.clear();
        match self.joins(&step) {
            true => self.join(step),
            false => self.done.push(step),
        }
        self.open = true;
    }

    /// Whether the step carries on a run of typing: one caret, and where it stopped.
    fn joins(&self, step: &Step) -> bool {
        let (Some(last), [change]) = (self.done.last(), step.as_slice()) else {
            return false;
        };
        self.open && matches!(last.as_slice(), [one] if one.continues(change))
    }

    fn join(&mut self, step: Step) {
        let Some([last]) = self.done.last_mut().map(|step| step.as_mut_slice()) else {
            return;
        };
        for change in step {
            last.inserted.push_str(&change.inserted);
        }
    }

    /// The changes that undo the last step, in the order to apply them.
    pub(crate) fn undo(&mut self) -> Option<Step> {
        let step = self.done.pop()?;
        self.undone.push(step.clone());
        self.open = false;
        Some(step.iter().rev().map(Change::inverse).collect())
    }

    /// The last undone step, moved back.
    pub(crate) fn redo(&mut self) -> Option<Step> {
        let step = self.undone.pop()?;
        self.done.push(step.clone());
        self.open = false;
        Some(step)
    }

    /// Ends the current typing run.
    pub(crate) fn close(&mut self) {
        self.open = false;
    }
}
