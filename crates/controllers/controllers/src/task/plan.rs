//! The plan: where the user puts a task among the ones waiting.

use groove_types::ExternalId;

use crate::{AppState, Services, Spawner};

/// Where a task lands: above `before`, or at the end of the side it is dropped on.
#[derive(Debug, Clone, PartialEq)]
pub struct Landing {
    pub external_id: ExternalId,
    pub before: Option<ExternalId>,
    pub later: bool,
}

/// One task moved in the plan, kept in the slice and written to disk.
pub(super) fn reorder(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    landing: &Landing,
) {
    let waiting = state.task.waiting(&state.session.worked());
    let shown = state.task.planned(&waiting);
    let order = groove_task_service::moved(
        &shown,
        &landing.external_id,
        landing.before.as_ref(),
        landing.later,
    );
    state.task.plan = order.clone();
    let service = services.task.clone();
    crate::spawn::record(spawner, async move { service.save(order).await });
}
