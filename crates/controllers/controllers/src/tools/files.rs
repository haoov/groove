//! What stands on the files themselves.

use groove_agent_service::Call;
use groove_agent_service::tools::answers;
use groove_workspace_service::{opened, opened_at};

use crate::{AppState, Continuation, Services, Spawner};

/// The file the user has open, and where their caret stands in it.
pub(super) fn open_file(state: &AppState, call: Call) {
    let Some(open) = &state.workspace.opened else {
        return call.reply.json(&answers::no_file());
    };
    let caret = open.new.caret();
    let commit = state.workspace.commit.as_ref().map(|one| one.sha.as_str());
    let (worktree, at) = (
        state.workspace.worktree.as_ref(),
        (caret.line, caret.column),
    );
    let said = answers::open_file(&open.path, worktree, commit, at, open.new.dirty());
    call.reply.json(&said);
}

/// The notes left on the session, the user's and the agent's.
pub(super) fn notes(state: &AppState, services: &Services, spawner: &dyn Spawner, call: Call) {
    let Some(open) = super::about(state, &call) else {
        return call.reply.failed(super::NO_SESSION);
    };
    let (session, service, reply) = (
        open.session.id.clone(),
        services.delivery.clone(),
        call.reply,
    );
    spawner.spawn(Box::pin(async move {
        let notes = service.notes(&session).await;
        Box::new(
            move |_: &mut AppState, _: &Services, _: &dyn Spawner| match notes {
                Ok(notes) => reply.json(&answers::annotations(&notes)),
                Err(e) => reply.failed(e.message),
            },
        ) as Continuation
    }));
}

/// One file of a worktree as it stands on disk, or as one commit left it.
pub(super) fn read(state: &AppState, spawner: &dyn Spawner, call: Call) {
    let Some(worktree) = super::worktree(state, &call) else {
        return call.reply.failed(super::NO_WORKTREE);
    };
    let Some(path) = call.text("path").map(str::to_string) else {
        return call.reply.failed("read_file needs the path of a file");
    };
    let commit = call.text("commit").map(str::to_string);
    let reply = call.reply;
    spawner.spawn(Box::pin(async move {
        let dir = std::path::PathBuf::from(&worktree.path);
        let read = match &commit {
            Some(sha) => opened_at(&dir, sha, &path).await,
            None => opened(&dir, &path, groove_workspace_service::HEAD).await,
        };
        Box::new(
            move |_: &mut AppState, _: &Services, _: &dyn Spawner| match read {
                Ok(file) => reply.json(&answers::file(&path, commit.as_deref(), &file.new.text())),
                Err(e) => reply.failed(e.message),
            },
        ) as Continuation
    }));
}
