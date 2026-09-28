use crate::{Day, ErrorKind, Forge, ProviderId, SessionId, SessionKind, Timestamp, WorktreeId};

#[test]
fn ids_are_transparent_strings_with_a_named_debug() {
    let id = SessionId::new("gh-groove-50");
    assert_eq!(id.to_string(), "gh-groove-50");
    assert_eq!(format!("{id:?}"), "SessionId(gh-groove-50)");
    assert_eq!(serde_json::to_string(&id).unwrap(), "\"gh-groove-50\"");
    let back: WorktreeId = serde_json::from_str("\"wt-1\"").unwrap();
    assert_eq!(back.as_str(), "wt-1");
}

#[test]
fn days_round_trip_through_the_epoch_count() {
    assert_eq!(Day::parse("1970-01-01").unwrap().days(), 0);
    assert_eq!(Day::parse("2000-03-01").unwrap().days(), 11_017);
    for text in ["1969-12-31", "2024-02-29", "2026-09-13", "2100-12-31"] {
        let day = Day::parse(text).unwrap();
        assert_eq!(Day::from_days(day.days()), day, "{text}");
        assert_eq!(day.to_string(), text);
    }
    assert_eq!(
        Day::parse("2023-02-29").unwrap_err().kind,
        ErrorKind::Invalid
    );
    assert!(Day::parse("2026-9-1x").is_err());
}

#[test]
fn a_timestamp_knows_its_day_and_age() {
    let t = Timestamp::new(1_788_886_029);
    assert_eq!(t.day().to_string(), "2026-09-08");
    let later = Timestamp::new(t.seconds() + 3 * 86_400 + 5);
    assert_eq!(t.days_before(later), 3);
    assert_eq!(later.age_at(t).as_secs(), 0);
    assert_eq!(
        Day::parse("2026-09-08").unwrap().plus_days(30).to_string(),
        "2026-10-08"
    );
}

#[test]
fn names_parse_both_ways() {
    assert_eq!(ProviderId::parse("github").unwrap(), ProviderId::Github);
    assert_eq!(
        ProviderId::parse("jira").unwrap_err().kind,
        ErrorKind::Invalid
    );
    assert_eq!(Forge::parse("gitlab").unwrap().as_str(), "gitlab");
    assert_eq!(SessionKind::Explorer.name(), "explorer");
    let json = serde_json::to_value(SessionKind::Review {
        project: "g/p".into(),
        iid: 7,
    })
    .unwrap();
    assert_eq!(json["kind"], "review");
    assert_eq!(json["iid"], 7);
}

#[test]
fn an_instant_a_forge_wrote_reads_back_as_seconds() {
    let noon = Timestamp::parse("2026-09-20T12:00:00Z").unwrap();
    assert_eq!(noon.day().to_string(), "2026-09-20");
    assert_eq!(noon.seconds() % 86_400, 12 * 3600);
}

#[test]
fn a_fraction_and_an_offset_are_both_understood() {
    let utc = Timestamp::parse("2026-09-20T12:00:00Z").unwrap();
    assert_eq!(Timestamp::parse("2026-09-20T12:00:00.482Z").unwrap(), utc);
    assert_eq!(Timestamp::parse("2026-09-20T14:00:00+02:00").unwrap(), utc);
    assert_eq!(Timestamp::parse("2026-09-20T09:00:00-03:00").unwrap(), utc);
    assert_eq!(Timestamp::parse("2026-09-20T12:00:00").unwrap(), utc);
}

#[test]
fn what_is_not_an_instant_is_refused() {
    assert!(Timestamp::parse("2026-09-20").is_err());
    assert!(Timestamp::parse("2026-09-20T25:00:00Z").is_err());
    assert!(Timestamp::parse("").is_err());
}

fn asked(project: &str, iid: u64, web_url: &str) -> crate::ReviewMr {
    crate::ReviewMr {
        forge: Forge::Github,
        project: project.into(),
        iid,
        title: "fix: one".into(),
        author: "someone".into(),
        source_branch: "fix/one".into(),
        target_branch: "main".into(),
        draft: false,
        web_url: web_url.into(),
        updated_at: Timestamp::new(0),
        local_path: None,
        approved: false,
        review: None,
    }
}

#[test]
fn an_mr_names_the_session_that_reviews_it_the_same_way_every_time() {
    let one = asked("acme/groove", 7, "https://github.com/acme/groove/pull/7");
    assert_eq!(one.session_id(), "review-acme-groove-7");
    assert_eq!(one.session_id(), asked("ACME/Groove", 7, "").session_id());
}

#[test]
fn an_mrs_own_page_says_where_its_repo_is_cloned_from() {
    let github = asked("acme/groove", 7, "https://github.com/acme/groove/pull/7");
    assert_eq!(
        github.clone_url().as_deref(),
        Some("https://github.com/acme/groove.git")
    );
    let gitlab = asked(
        "devops/charts",
        3,
        "https://gitlab.example.com/devops/charts/-/merge_requests/3",
    );
    assert_eq!(
        gitlab.clone_url().as_deref(),
        Some("https://gitlab.example.com/devops/charts.git")
    );
    assert_eq!(asked("a/b", 1, "").clone_url(), None);
}

#[test]
fn a_pooled_clone_holds_only_the_project_whose_whole_path_it_ends_with() {
    let entry = crate::PoolEntry {
        slug: "gitlab.example.com/wiremind/big/p".into(),
        path: std::path::PathBuf::from("/pool/p"),
    };
    assert!(entry.holds("wiremind/big/p"), "its own path");
    assert!(entry.holds("big/p"), "the tail of it, on a segment");
    assert!(!entry.holds("g/p"), "not half a segment");
    assert!(!entry.holds("other/p"));
    let bare = crate::PoolEntry {
        slug: "acme/groove".into(),
        path: std::path::PathBuf::from("/pool/groove"),
    };
    assert!(bare.holds("acme/groove"), "the whole slug");
}
