//! What the task capability asks of the plan on disk.

use groove_ledger::{Ledger, Stored as _};
use groove_plan::{Placed, Plan};
use groove_types::{Day, Error, ExternalId, Result, TimeSummary};

/// The service, cheap to clone: a handle on the plan and on the ledger.
#[derive(Clone)]
pub struct Service {
    plan: Plan,
    ledger: Ledger,
}

impl Service {
    pub fn new(plan: Plan, ledger: Ledger) -> Self {
        Self { plan, ledger }
    }

    /// A service on a private in-memory database, for tests.
    pub async fn in_memory() -> Result<Self> {
        let plan = Plan::in_memory().await;
        let ledger = Ledger::in_memory().await;
        let store = |e| Error::store("in-memory database", e);
        Ok(Self::new(plan.map_err(store)?, ledger.map_err(store)?))
    }

    /// What every task has measured, and how much of it the source has been told.
    pub async fn time(&self) -> Result<Vec<(ExternalId, TimeSummary)>> {
        let today = groove_types::Timestamp::now().day();
        self.ledger.summaries(today).await
    }

    /// The seconds the clock measured, task by task.
    pub async fn credit(&self, owed: Vec<(ExternalId, i64)>, day: Day) -> Result<()> {
        for (id, seconds) in owed {
            self.ledger.credit(&id, seconds, day).await?;
        }
        Ok(())
    }

    pub async fn logged(&self, id: &ExternalId, seconds: i64) -> Result<()> {
        self.ledger.logged(id, seconds).await
    }

    pub async fn order(&self) -> Result<Vec<Placed>> {
        self.plan.order().await
    }

    pub async fn save(&self, order: Vec<Placed>) -> Result<()> {
        self.plan.save(&order).await
    }
}
