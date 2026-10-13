//! The review queue, read from every host the pool knows.

use crate::{AppState, Continuation, Services, Spawner};

pub(super) fn read(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    let session = services.session.clone();
    let job = state.begin("reading the review queue");
    spawner.spawn(Box::pin(async move {
        let pool = session.list_pool().await;
        let (found, failed) = match &pool {
            Ok(pool) => groove_delivery_service::Service::review_queue(pool).await,
            Err(_) => Default::default(),
        };
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            if let Err(e) = pool {
                return state.failed(e);
            }
            state.delivery.reviews = found;
            state.delivery.reviews_read = true;
            for one in failed {
                state.failed(one);
            }
        }) as Continuation
    }));
}
