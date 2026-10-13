//! How long a stream waits before it opens again: doubling while it fails, parked on an error a retry does not fix.

use std::time::Duration;

use groove_kube::Error;

pub(crate) const FIRST_WAIT: Duration = Duration::from_secs(1);
const LONGEST_WAIT: Duration = Duration::from_secs(30);
const PARKED: Duration = Duration::from_secs(5 * 60);

#[derive(Debug)]
pub(crate) struct Backoff(Duration);

impl Backoff {
    pub(crate) fn new() -> Self {
        Self(FIRST_WAIT)
    }

    pub(crate) fn after(&mut self, error: &Error) -> Duration {
        match lasting(error) {
            true => PARKED,
            false => self.grow(),
        }
    }

    /// As `after`, but a followed log waits on a 400 as on a transient error.
    pub(crate) fn after_logs(&mut self, error: &Error, previous: bool) -> Duration {
        match error {
            Error::Api { status: 400, .. } if !previous => self.grow(),
            _ => self.after(error),
        }
    }

    /// The wait now; the next one doubles.
    pub(crate) fn grow(&mut self) -> Duration {
        let wait = self.0;
        self.0 = (wait * 2).min(LONGEST_WAIT);
        wait
    }

    /// The stream opened: the next failure waits the least again.
    pub(crate) fn reset(&mut self) {
        self.0 = FIRST_WAIT;
    }
}

/// A bad request, a refusal of the kind or the verb, a kind gone, or a context gone.
fn lasting(error: &Error) -> bool {
    matches!(
        error,
        Error::Api {
            status: 400 | 403 | 404 | 405 | 422,
            ..
        } | Error::NoContext(_)
            | Error::Kubeconfig(_)
    )
}
