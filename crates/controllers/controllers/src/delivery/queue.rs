//! The review queue, read from every host the pool knows.

use crate::{AppState, Continuation, Services, Spawner};

pub(super) fn read(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    let session = services.session.clone();
    let job = state.begin("reading the review queue");
    spawner.spawn(Box::pin(async move {
        let pool = session.list_pool();
        let (found, failed) = groove_delivery_service::Service::review_queue(&pool).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            state.delivery.reviews = found;
            for one in failed {
                state.failed(one);
            }
        }) as Continuation
    }));
}
