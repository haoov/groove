use crate::Stored as _;
use groove_types::ExternalId;

use crate::{Placed, Plan};

async fn plan() -> Plan {
    Plan::in_memory().await.expect("a plan")
}

fn placed(id: &str) -> Placed {
    Placed::new(ExternalId::new(id))
}

#[tokio::test]
async fn the_order_comes_back_as_it_was_saved() {
    let plan = plan().await;
    let order = vec![placed("b"), placed("a"), placed("c")];
    plan.save(&order).await.expect("the plan is written");
    assert_eq!(plan.order().await.expect("the plan is read"), order);
}

#[tokio::test]
async fn a_saved_order_replaces_the_one_before_it() {
    let plan = plan().await;
    plan.save(&[placed("a"), placed("b")])
        .await
        .expect("the first plan");
    plan.save(&[placed("b")]).await.expect("the second");
    assert_eq!(plan.order().await.expect("the plan is read"), [placed("b")]);
}

#[tokio::test]
async fn an_empty_plan_reads_as_no_order_at_all() {
    let plan = plan().await;
    assert!(plan.order().await.expect("the plan is read").is_empty());
}
