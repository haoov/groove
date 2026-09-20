//! What the task capability asks of the plan on disk.

use groove_plan::{Placed, Plan};
use groove_types::Result;

/// The service, cheap to clone: a handle on the plan.
#[derive(Clone)]
pub struct Service {
    plan: Plan,
}

impl Service {
    pub fn new(plan: Plan) -> Self {
        Self { plan }
    }

    /// A service on a private in-memory database, for tests.
    pub async fn in_memory() -> Result<Self> {
        Ok(Self::new(Plan::in_memory().await?))
    }

    pub async fn order(&self) -> Result<Vec<Placed>> {
        self.plan.order().await
    }

    pub async fn save(&self, order: Vec<Placed>) -> Result<()> {
        self.plan.save(&order).await
    }
}
