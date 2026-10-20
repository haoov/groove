use groove_types::{FollowKey, Timestamp};

use super::pod_kind;
use crate::Follows;

fn pod(name: &str) -> (String, String, String) {
    ("kind".into(), "paxone".into(), name.into())
}

#[test]
fn a_pod_no_tab_reads_keeps_no_usage_and_its_namespace_no_revision() {
    let mut follows = Follows::default();
    let key = FollowKey::named("kind", &pod_kind(), Some("paxone"), "api-0");
    follows.lease(&key, "resource");
    follows.set_usage(pod("api-0"), None, Timestamp::new(0));
    follows.set_helm(pod("api"), Some(3));
    let released = follows.release("resource");
    follows.drop_unread(&key, released[0].1);
    assert!(follows.usage(&pod("api-0")).is_none());
    assert!(follows.helm(&pod("api")).is_none());
}

#[test]
fn a_usage_read_that_keeps_failing_is_said_once() {
    let mut follows = Follows::default();
    assert!(follows.fail_usage(pod("api-0"), Timestamp::new(0)));
    assert!(!follows.fail_usage(pod("api-0"), Timestamp::new(15)));
    assert_eq!(
        follows.usage(&pod("api-0")),
        Some(&(None, Timestamp::new(15)))
    );
    follows.set_usage(pod("api-0"), None, Timestamp::new(30));
    assert!(follows.fail_usage(pod("api-0"), Timestamp::new(45)));
}
