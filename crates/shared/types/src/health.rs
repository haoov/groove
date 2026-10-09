//! What a status or a ready cell of a Table says of an object's health.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Good,
    /// On its way: pending, creating, terminating, short of ready.
    Waiting,
    Failing,
    /// Ran and ended: completed, succeeded.
    Done,
}

const FAILING: [&str; 12] = [
    "CrashLoopBackOff",
    "Error",
    "Failed",
    "ImagePullBackOff",
    "ErrImagePull",
    "OOMKilled",
    "Evicted",
    "CreateContainerConfigError",
    "CreateContainerError",
    "Degraded",
    "Missing",
    "Lost",
];
const WAITING: [&str; 8] = [
    "Pending",
    "ContainerCreating",
    "PodInitializing",
    "Terminating",
    "Progressing",
    "OutOfSync",
    "Unknown",
    "Suspended",
];
const GOOD: [&str; 8] = [
    "Running",
    "Ready",
    "Bound",
    "Active",
    "Available",
    "Healthy",
    "Synced",
    "True",
];
const DONE: [&str; 2] = ["Completed", "Succeeded"];

/// A status word; `Init:Error` reads as its part after the colon. `None` for a word it does not know.
pub fn status_health(text: &str) -> Option<Health> {
    let word = text.rsplit(':').next().unwrap_or(text).trim();
    let held = |list: &[&str]| list.contains(&word);
    match () {
        _ if held(&FAILING) => Some(Health::Failing),
        _ if text.starts_with("Init:") || held(&WAITING) => Some(Health::Waiting),
        _ if held(&GOOD) => Some(Health::Good),
        _ if held(&DONE) => Some(Health::Done),
        _ => None,
    }
}

/// `ready/wanted`: all of them good, none failing, some waiting; none of none ended.
pub fn ready_health(text: &str) -> Option<Health> {
    let (ready, wanted) = text.split_once('/')?;
    let (ready, wanted): (u32, u32) = (ready.trim().parse().ok()?, wanted.trim().parse().ok()?);
    Some(match (ready, wanted) {
        (_, 0) => Health::Done,
        (ready, wanted) if ready >= wanted => Health::Good,
        (0, _) => Health::Failing,
        _ => Health::Waiting,
    })
}
