//! What the environment check found: each program the app runs, and whether it can.

/// One program the app shells out to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    pub name: &'static str,
    /// What stops working without it.
    pub purpose: &'static str,
    /// Without it the app cannot work at all.
    pub required: bool,
    /// `None` when it does not run.
    pub found: Option<Found>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub version: String,
    /// `None` for a program with no account.
    pub signed_in: Option<bool>,
}

impl Tool {
    /// It runs, and it is signed in where it has an account.
    pub fn ready(&self) -> bool {
        self.found
            .as_ref()
            .is_some_and(|found| found.signed_in != Some(false))
    }
}
