//! The `agent` controller: one function per user action on the `agent` service.

mod chat;
pub mod skills;
mod start;

use groove_types::{ApprovalId, SessionId};

use crate::{AppState, Services, Spawner};

pub use chat::hooked;
pub use start::start;
pub(crate) use start::{asking, launch_dir, rewrite};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `agent.start`: launch the session's agent at the worktree root, headless.
    Start { session: SessionId },
    /// `agent.end`: end the agent and forget it.
    End { session: SessionId },
    /// `agent.say`: words from Groove's own prompt, sent to the agent as a prompt.
    Say { session: SessionId, text: String },
    /// `agent.model`: the agent's next answers come from `model`.
    Model { session: SessionId, model: String },
    /// `agent.interrupt`: the agent's turn stopped where it stands.
    Interrupt { session: SessionId },
    /// `agent.answer`: the tool the agent asked for runs, on the picked answers, or does not.
    Answer {
        session: SessionId,
        id: String,
        allow: bool,
        picked: Vec<String>,
    },
    /// `agent.approve`: the write the agent asked for runs.
    Approve { id: ApprovalId },
    /// `agent.refuse`: it does not, and its agent hears so.
    Refuse { id: ApprovalId },
    /// `agent.auto_approve`: every write of this session runs without asking.
    AutoApprove { session: SessionId, on: bool },
    /// `agent.reload`: the agent ended and started again, resuming its own thread.
    Reload { session: SessionId },
    /// `agent.list_skills`: both plugins written, then what they offer.
    ListSkills,
    /// `agent.send_skill`: one skill sent to the agent as its prompt.
    SendSkill {
        session: SessionId,
        id: String,
        args: Option<String>,
    },
    /// `agent.delete_skill`: one skill of the user's own.
    DeleteSkill { id: String },
    /// `agent.switch_skill`: one skill given to sessions, or no longer; a core one stays on.
    SwitchSkill { id: String, on: bool },
    /// `agent.run_routine`: one routine run now, by its button.
    RunRoutine { id: String },
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Start { .. } => "agent.start",
            Command::End { .. } => "agent.end",
            Command::Say { .. } => "agent.say",
            Command::Model { .. } => "agent.model",
            Command::Interrupt { .. } => "agent.interrupt",
            Command::Answer { .. } => "agent.answer",
            Command::Approve { .. } => "agent.approve",
            Command::Refuse { .. } => "agent.refuse",
            Command::AutoApprove { .. } => "agent.auto_approve",
            Command::Reload { .. } => "agent.reload",
            Command::ListSkills => "agent.list_skills",
            Command::SendSkill { .. } => "agent.send_skill",
            Command::DeleteSkill { .. } => "agent.delete_skill",
            Command::SwitchSkill { .. } => "agent.switch_skill",
            Command::RunRoutine { .. } => "agent.run_routine",
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::Start { session } => start(state, spawner, session),
        Command::End { session } => end(state, &session),
        Command::Say { session, text } => chat::say(state, &session, &text),
        Command::Model { session, model } => chat::model(state, &session, model),
        Command::Interrupt { session } => chat::interrupt(state, &session),
        Command::Answer {
            session,
            id,
            allow,
            picked,
        } => chat::answer(state, &session, &id, (allow, &picked)),
        Command::Approve { id } => crate::tools::allow(state, services, spawner, &id),
        Command::Refuse { id } => crate::tools::refuse(state, &id),
        Command::AutoApprove { session, on } => {
            auto_approve(state, services, spawner, &session, on)
        }
        Command::Reload { session } => reload(state, spawner, session),
        Command::ListSkills => skills::list(state, spawner),
        Command::SendSkill { session, id, args } => {
            skills::send(state, &session, &id, args.as_deref())
        }
        Command::DeleteSkill { id } => skills::delete(state, spawner, &id),
        Command::SwitchSkill { id, on } => skills::switch(state, services, spawner, (id, on)),
        Command::RunRoutine { id } => crate::routine::button(state, services, spawner, &id),
    }
}

/// The agent ended and started again. The launch resumes its own thread by itself.
fn reload(state: &mut AppState, spawner: &dyn Spawner, session: SessionId) {
    end(state, &session);
    start(state, spawner, session);
}

/// Every write of this session runs without asking, or waits again.
fn auto_approve(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    session: &SessionId,
    on: bool,
) {
    if let Some(open) = state.session.get_mut(session) {
        open.state.auto_approve = on;
    }
    crate::session::set_auto_approve(services, spawner, session, on);
}

/// The session's launch files taken away; a start asked meanwhile runs after.
pub(crate) fn forget(state: &mut AppState, spawner: &dyn Spawner, session: &SessionId) {
    let (dir, id) = (launch_dir(state), session.clone());
    let write = async move { groove_agent_service::forget(&dir, id.as_str()).await };
    let id = session.clone();
    rewrite(
        state,
        spawner,
        session.clone(),
        write,
        move |state, spawner, held| {
            if let Some(prompt) = held {
                asking(state, spawner, id, prompt);
            }
        },
    );
}

pub fn end(state: &mut AppState, session: &SessionId) {
    crate::tools::drop_asks(state, session);
    if let Some(stream) = state.agent.end(session).and_then(|one| one.stream) {
        let _ = stream.terminate();
    }
}
