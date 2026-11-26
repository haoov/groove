//! The delivery controller, one file per feature.

mod queue;
mod threads;

use crate::tests::fixture::{pooled_clone, until, worktree};
use crate::{Command as Cmd, SyncSpawner, delivery, dispatch};
