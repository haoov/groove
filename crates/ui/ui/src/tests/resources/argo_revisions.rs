//! An Application's sync sections: the commits its sources stand at, and its sync policy.

use groove_controllers::Command;
use groove_controllers::cluster::Command as Cluster;
use groove_types::RevisionMeta;

use super::argo::failing;
use super::argo_sync::{hit, opened_application};
use super::fixture::runs;
use crate::hit::Target;
use crate::tests::window;
use crate::views::session::resources::View;

const SYNCED: &str = "e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c3d2e1f0";
const TARGET: &str = "d21b77a0c5d4e3f2a1b0c9d8e7f6a5b4c3d2e1f0";

fn revised(sha: &str) -> groove_controllers::cluster_service::Revised {
    let app = ("staging".into(), "shop".into(), "billing-acc".into());
    (app, None, sha.into())
}

#[test]
fn an_out_of_sync_source_asks_argo_cd_for_both_commits_and_shows_their_messages() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.sync = "OutOfSync".into();
    part.sources[0].synced = Some(SYNCED.into());
    part.sources[0].compared = Some(TARGET.into());
    let (mut app, mut ui) = opened_application(object);
    hit(&app, &mut ui, Target::ResourceView(View::Describe));
    let asked = crate::frame_commands(&app, &ui, window());
    for sha in [TARGET, SYNCED] {
        let revised = Box::new(revised(sha));
        let ask = Command::Cluster(Cluster::ArgoRevision { revised });
        assert!(asked.contains(&ask), "{sha}: {asked:?}");
    }

    let revisions = &mut app.cluster.store.revisions;
    let meta = |message: &str| RevisionMeta {
        author: "Jane Doe".into(),
        date: None,
        message: message.into(),
    };
    for (sha, said) in [(TARGET, "bump the api"), (SYNCED, "add the export job")] {
        revisions.begin(&revised(sha));
        revisions.set(revised(sha), Ok(meta(said)));
    }
    let texts: Vec<String> = runs(&app, &ui).into_iter().map(|one| one.text).collect();
    let shown = ["TARGET", "d21b77a", "bump the api", "SYNCED"];
    for shown in shown.into_iter().chain(["add the export job"]) {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
    let heading = texts.iter().find(|one| one.starts_with("OUT OF SYNC"));
    let heading = heading.unwrap_or_else(|| panic!("the state as heading: {texts:?}"));
    assert!(heading.ends_with("TARGET MOVED"), "{heading}");
    let asked = crate::frame_commands(&app, &ui, window());
    let again = asked
        .iter()
        .any(|one| matches!(one, Command::Cluster(Cluster::ArgoRevision { .. })));
    assert!(!again, "a commit read is not asked again");
}

#[test]
fn a_source_at_its_target_shows_one_row_and_a_refusal_in_place_of_the_message() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.sources[0].synced = Some(SYNCED.into());
    part.sources[0].compared = Some(SYNCED.into());
    let (mut app, mut ui) = opened_application(object);
    hit(&app, &mut ui, Target::ResourceView(View::Describe));
    let revisions = &mut app.cluster.store.revisions;
    revisions.begin(&revised(SYNCED));
    revisions.set(revised(SYNCED), Err("permission denied".into()));
    let texts: Vec<String> = runs(&app, &ui).into_iter().map(|one| one.text).collect();
    for shown in ["SYNCED", "permission denied"] {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
    assert!(!texts.iter().any(|one| one == "TARGET"), "{texts:?}");
}

#[test]
fn the_sync_policy_holds_the_options_the_retry_and_the_windows_the_summary_not_auto_sync() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.retry = Some(groove_types::Retry {
        limit: 5,
        backoff: Some("5s".into()),
        factor: Some(2),
        max: Some("3m".into()),
    });
    let (mut app, mut ui) = opened_application(object);
    hit(&app, &mut ui, Target::ResourceView(View::Describe));
    let named = (
        "staging".to_string(),
        "shop".to_string(),
        "billing-acc".to_string(),
    );
    let asked = crate::frame_commands(&app, &ui, window());
    let ask = Command::Cluster(Cluster::ArgoWindows { app: named.clone() });
    assert!(asked.contains(&ask), "{asked:?}");
    let deny = groove_types::SyncWindow {
        kind: "deny".into(),
        schedule: "0 22 * * *".into(),
        duration: "8h".into(),
        ..Default::default()
    };
    let windows = groove_types::SyncWindows {
        can_sync: false,
        assigned: vec![deny.clone()],
        active: vec![deny],
    };
    let now = groove_types::Timestamp::now();
    app.cluster.store.windows.begin(&named);
    app.cluster.store.windows.set(named, Ok(windows), now);
    assert!(
        !crate::frame_commands(&app, &ui, window()).contains(&ask),
        "read just now"
    );
    let texts: Vec<String> = runs(&app, &ui).into_iter().map(|one| one.text).collect();
    let shown = [
        "shut · deny 0 22 * * * for 8h, active",
        "SYNC POLICY",
        "on · prune on · self-heal on",
        "ServerSideApply=true",
        "5 times · 5s backoff ×2 up to 3m",
    ];
    for shown in shown {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
    assert!(!texts.iter().any(|one| one == "auto-sync"), "{texts:?}");
}
