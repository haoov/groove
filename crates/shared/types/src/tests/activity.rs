//! An agent's activity and the attention class it falls in.

use crate::{AgentStatus, ApprovalId, Ask, AttentionClass, SessionActivity, Timestamp};

fn activity(status: AgentStatus) -> SessionActivity {
    SessionActivity {
        status,
        tool: None,
        asks: Vec::new(),
        changed_at: Timestamp::new(0),
        seen_at: None,
    }
}

#[test]
fn the_attention_class_follows_asks_then_status() {
    assert_eq!(
        activity(AgentStatus::Working).class(),
        AttentionClass::Moving
    );
    assert_eq!(activity(AgentStatus::Idle).class(), AttentionClass::Quiet);
    assert_eq!(
        activity(AgentStatus::Done { seen: false }).class(),
        AttentionClass::ActWhenYouLook
    );
    assert_eq!(
        activity(AgentStatus::Done { seen: true }).class(),
        AttentionClass::Quiet
    );
    assert_eq!(
        activity(AgentStatus::Exited { code: 1 }).class(),
        AttentionClass::NeedsYou
    );
    let mut asked = activity(AgentStatus::Working);
    asked.asks.push(Ask {
        id: ApprovalId::new("a"),
        op: "git.commit".into(),
        subject: "feat: x".into(),
        text: String::new(),
        worktree: None,
    });
    assert_eq!(asked.class(), AttentionClass::NeedsYou);
}
