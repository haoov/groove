//! The delivery controller, one file per feature.

mod notes;
mod queue;

use crate::tests::fixture::{pooled_clone, send, until, worktree};
use crate::{Command as Cmd, SyncSpawner, delivery, dispatch};
