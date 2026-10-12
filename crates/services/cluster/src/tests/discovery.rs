use crate::Store;

#[test]
fn a_sign_in_frees_a_waiting_discovery_but_not_a_running_one() {
    let mut store = Store::default();
    assert!(store.begin_discovery("kind"));
    store.refuse_discovery("kind");
    store.signed_in("kind");
    assert!(!store.discovering("kind"), "the waiting one may run now");
    assert!(store.begin_discovery("kind"));
    let (failures, _) = store.refuse_discovery("kind");
    store.retry_discovery("kind", failures);
    assert!(store.begin_discovery("kind"), "the retry runs");
    store.signed_in("kind");
    assert!(!store.begin_discovery("kind"), "one runs already");
}
