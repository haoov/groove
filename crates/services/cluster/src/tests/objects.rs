//! One object followed by its readers.

use super::*;

#[test]
fn a_followed_object_lands_changes_and_goes_with_its_last_reader() {
    use crate::{Change, Followed};
    use groove_types::{Described, FollowKey};
    let described = |uid: &str, name: &str| Described {
        uid: uid.into(),
        name: name.into(),
        namespace: Some("shop".into()),
        labels: Vec::new(),
        annotations: Vec::new(),
        created: None,
        owners: Vec::new(),
        replicas: None,
        selector: Vec::new(),
        pod: None,
        app: None,
        event: None,
        yaml: "".into(),
    };
    let mut store = crate::Store::default();
    let key = FollowKey::named("kind", &pods("shop").kind, Some("shop"), "api-0");
    assert!(store.follows.lease(&key, "tab").is_some());
    assert!(
        store.follows.lease(&key, "tab").is_none(),
        "one watcher a key"
    );
    store
        .follows
        .apply(&key, Followed::Reset(vec![described("u1", "api-0")]));
    store.follows.apply(
        &key,
        Followed::Changes(vec![
            Change::Put(Box::new(described("u1", "api-0-renamed"))),
            Change::Gone("u9".into()),
        ]),
    );
    let held = store.follows.get(&key).expect("followed");
    assert!(held.synced);
    assert_eq!(held.objects.len(), 1);
    assert_eq!(held.objects[0].name, "api-0-renamed");
    assert_eq!(store.follows.read_by("tab").count(), 1);
    assert_eq!(store.follows.release("tab"), [(key.clone(), 1)]);
    store.follows.drop_unread(&key, 1);
    assert!(store.follows.get(&key).is_none());
}

fn described(yaml: &str) -> groove_types::Described {
    groove_types::Described {
        uid: "u1".into(),
        name: "api-0".into(),
        namespace: Some("shop".into()),
        labels: Vec::new(),
        annotations: Vec::new(),
        created: None,
        owners: Vec::new(),
        replicas: None,
        selector: Vec::new(),
        pod: None,
        app: None,
        event: None,
        yaml: yaml.into(),
    }
}

#[test]
fn an_object_followed_by_name_reads_as_a_read_only_text_that_keeps_its_caret() {
    use crate::Followed;
    use groove_types::{Caret, Edit, FollowKey, Motion};
    let mut store = crate::Store::default();
    let key = FollowKey::named("kind", &pods("shop").kind, Some("shop"), "api-0");
    store.follows.lease(&key, "tab");
    store.follows.apply(
        &key,
        Followed::Reset(vec![described("metadata:\n  name: api-0\n")]),
    );
    let yamls = &mut store.follows.yamls;
    assert_eq!(yamls.get(&key).map(|one| one.lines()), Some(2));
    assert!(yamls.edit(&key, &Edit::Move(Motion::Down)));
    assert!(yamls.edit(&key, &Edit::Extend(Motion::LineEnd)));
    assert!(
        !yamls.edit(&key, &Edit::Insert("x".into())),
        "nothing writes"
    );
    assert_eq!(
        yamls.get(&key).map(|one| one.selected()),
        Some("  name: api-0".into())
    );
    let changed = described("metadata:\n  name: api-0\nstatus:\n  phase: Running\n");
    store.follows.apply(&key, Followed::Reset(vec![changed]));
    let buffer = store.follows.yamls.get(&key).expect("the text");
    assert_eq!(
        (buffer.lines(), buffer.caret()),
        (4, Caret::new(1, 13)),
        "the caret stays"
    );
    store.follows.release("tab");
    store.follows.drop_unread(&key, 1);
    assert!(store.follows.yamls.get(&key).is_none());
}

#[test]
fn a_draft_takes_every_edit_and_stands_while_the_object_moves_on() {
    use crate::Followed;
    use groove_types::{Edit, FollowKey, Motion};
    let mut store = crate::Store::default();
    let key = FollowKey::named("kind", &pods("shop").kind, Some("shop"), "api-0");
    store.follows.lease(&key, "tab");
    let first = described("spec:\n  replicas: 1\n");
    store.follows.apply(&key, Followed::Reset(vec![first]));
    let yamls = &mut store.follows.yamls;
    assert!(yamls.write(&key));
    let end = Motion::To(groove_types::Caret::new(2, 0));
    yamls.edit(&key, &Edit::Move(end));
    assert!(yamls.edit(&key, &Edit::Insert("  paused: true\n".into())));
    assert!(yamls.draft(&key).is_some_and(|one| one.edited()));
    let moved = described("spec:\n  replicas: 2\n");
    store.follows.apply(&key, Followed::Reset(vec![moved]));
    let yamls = &mut store.follows.yamls;
    let text = yamls.get(&key).map(|one| one.text()).unwrap_or_default();
    let written = "spec:\n  replicas: 1\n  paused: true\n";
    assert_eq!(text, written, "the draft stands");
    assert!(yamls.moved(&key));
    yamls.read_only(&key);
    let text = yamls.get(&key).map(|one| one.text()).unwrap_or_default();
    assert_eq!(text, "spec:\n  replicas: 2\n", "the object as it stands");
}
