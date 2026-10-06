use groove_types::{KubeAuth, KubeContext, Login};

use crate::{Event, State, apply};

fn context(name: &str) -> KubeContext {
    KubeContext {
        name: name.into(),
        cluster: name.into(),
        server: None,
        namespace: None,
        auth: KubeAuth::Token,
    }
}

#[test]
fn a_second_scan_waits_for_the_first() {
    let mut state = State::default();
    assert!(state.begin_scan());
    assert!(!state.begin_scan());
    apply(&mut state, Event::Found(vec![context("kind")]));
    assert!(state.begin_scan());
}

#[test]
fn a_scan_that_cannot_read_keeps_what_the_last_one_found() {
    let mut state = State::default();
    apply(&mut state, Event::Found(vec![context("kind")]));
    state.begin_scan();
    apply(&mut state, Event::Unread);
    assert_eq!(state.found, Some(vec![context("kind")]));
    assert!(!state.scanning);
}

#[test]
fn a_check_replaces_the_last_login_of_its_context_only() {
    let mut state = State::default();
    let refused = Login::Refused("token expired".into());
    let signed_in = Login::SignedIn {
        version: "v1.33.4".into(),
    };
    apply(
        &mut state,
        Event::Checked {
            context: "hub".into(),
            login: refused,
        },
    );
    assert!(state.begin_check("hub"));
    assert!(!state.begin_check("hub"));
    apply(
        &mut state,
        Event::Checked {
            context: "hub".into(),
            login: signed_in.clone(),
        },
    );
    assert_eq!(state.login("hub"), Some(&signed_in));
    assert_eq!(state.login("elpis"), None);
    assert!(!state.checking("hub"));
}
