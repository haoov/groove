//! What a menu row does, apart from the words it shows.

/// One action a menu offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    DiscardChanges,
    Close,
    CloseOthers,
    CloseAll,
    Note,
    NewFile,
    NewDirectory,
    Rename,
    Copy,
    Delete,
}

impl Act {
    /// The words its row shows.
    pub fn label(self) -> &'static str {
        match self {
            Act::DiscardChanges => "discard changes",
            Act::Close => "close",
            Act::CloseOthers => "close others",
            Act::CloseAll => "close all",
            Act::Note => "note",
            Act::NewFile => "new file",
            Act::NewDirectory => "new directory",
            Act::Rename => "rename",
            Act::Copy => "copy",
            Act::Delete => "delete",
        }
    }
}
