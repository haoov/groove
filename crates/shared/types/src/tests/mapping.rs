use crate::{Kind, Mapped, Mapping, NotionConfig, Priority, StatusIntent};

fn notion() -> NotionConfig {
    NotionConfig::bare("secret", "DB", "ME")
}

#[test]
fn a_name_takes_only_a_property_of_its_type() {
    assert!(Mapped::Due.fits(Kind::Date) && !Mapped::Due.fits(Kind::Number));
    assert!(Mapped::Status.fits(Kind::Status) && Mapped::Status.fits(Kind::Select));
    assert!(Mapped::Assignee.fits(Kind::People) && Mapped::Sprint.fits(Kind::Relation));
}

#[test]
fn each_of_groove_s_statuses_takes_one_value_of_the_source() {
    let mut config = notion();
    config.map(&Mapping::Name(Mapped::Status, "Status".into()));
    config.map(&Mapping::Status(
        StatusIntent::Ready,
        "To be defined".into(),
    ));
    config.map(&Mapping::Status(
        StatusIntent::Ready,
        "Ready for sprint".into(),
    ));
    config.map(&Mapping::Status(StatusIntent::Done, "Done".into()));
    let map = &config.status_map;
    assert_eq!(map.ready, ["Ready for sprint"], "the last one picked");
    assert_eq!(map.label(StatusIntent::Done), Some("Done"));
    assert_eq!(
        map.label(StatusIntent::InProgress),
        None,
        "a gap until picked"
    );
}

#[test]
fn a_status_pointed_elsewhere_forgets_its_values() {
    let mut config = notion();
    config.map(&Mapping::Name(Mapped::Status, "Status".into()));
    config.map(&Mapping::Status(StatusIntent::Done, "Done".into()));
    config.filters.exclude_statuses = vec!["Done".into()];
    config.map(&Mapping::Name(Mapped::Status, "State".into()));
    assert!(config.status_map.done.is_empty() && config.filters.exclude_statuses.is_empty());
}

#[test]
fn each_level_takes_one_value_and_the_required_names_are_held() {
    let mut config = notion();
    config.map(&Mapping::Name(Mapped::Priority, "Priority".into()));
    config.map(&Mapping::Priority(Priority::High, "P1".into()));
    assert_eq!(config.priority_map.value(Priority::High), Some("P1"));
    assert_eq!(config.priority_map.value(Priority::Low), None);
    config.map(&Mapping::Name(Mapped::Assignee, "Assignee".into()));
    config.map(&Mapping::Name(Mapped::Sprint, "Sprint".into()));
    assert!(config.unmapped().is_empty());
}
