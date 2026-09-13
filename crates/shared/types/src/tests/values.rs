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
