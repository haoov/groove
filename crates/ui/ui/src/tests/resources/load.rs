//! What the tab asks of the clusters: nothing for a context gone, nothing for a search still typed.

use super::*;

#[test]
fn a_label_word_still_typed_asks_for_no_watcher_until_the_search_is_left() {
    let (app, mut ui) = listing();
    ui.session.resources.typing = true;
    for typed in ["a", "app", "app=", "app=api"] {
        ui.session.resources.search.set(typed);
        assert!(clustered(&app, &ui).is_empty(), "{typed}");
    }
    ui.session.resources.typing = false;
    let asked = clustered(&app, &ui);
    let selected = asked.iter().filter_map(|one| match one {
        Command::Cluster(cluster::Command::Watch { key, .. }) => key.selector.as_deref(),
        _ => None,
    });
    assert_eq!(selected.collect::<Vec<_>>(), ["app=api", "app=api"]);
}

#[test]
fn a_context_removed_from_groove_is_neither_discovered_nor_watched() {
    let (mut app, ui) = listing();
    let open = app
        .session
        .get_mut(&SessionId::new("a"))
        .expect("the session");
    open.clusters.push(on("gone", Some("paxone")));
    assert!(clustered(&app, &ui).is_empty(), "no discovery of gone");
    app.cluster.store.set_kinds("gone", vec![pods()]);
    assert!(clustered(&app, &ui).is_empty(), "no watcher on gone");
}
