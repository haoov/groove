use crate::{ApprovalId, SessionId, Timestamp};

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    Ui,
    Mcp,
}

/// A queued write: `op` names the controller, `payload` its arguments.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Approval {
    pub id: ApprovalId,
    pub session: Option<SessionId>,
    pub op: String,
    pub payload: serde_json::Value,
    pub origin: Origin,
    pub created_at: Timestamp,
    pub claimed_at: Option<Timestamp>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Approved,
    Refused,
}
