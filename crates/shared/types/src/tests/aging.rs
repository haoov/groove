use crate::{Aging, TableColumn, Timestamp, agings, human_age};

#[test]
fn an_age_reads_as_kubectl_writes_it() {
    let ages = [
        (0, "0s"),
        (119, "119s"),
        (120, "2m"),
        (179, "2m59s"),
        (599, "9m59s"),
        (600, "10m"),
        (10_799, "179m"),
        (12_600, "3h30m"),
        (28_800, "8h"),
        (172_799, "47h"),
        (190_800, "2d5h"),
        (691_200, "8d"),
        (63_072_000, "2y"),
        (63_936_000, "2y10d"),
        (252_288_000, "8y"),
    ];
    for (seconds, said) in ages {
        assert_eq!(human_age(seconds), said, "{seconds}s");
    }
}

fn column(name: &str, date: bool) -> TableColumn {
    TableColumn {
        name: name.into(),
        priority: 0,
        date,
    }
}

#[test]
fn the_age_grows_from_the_creation_and_the_other_time_cells_from_their_arrival() {
    let columns = [
        column("Name", false),
        column("Restarts", false),
        column("Last Schedule", false),
        column("Synced", true),
        column("Status", false),
        column("Age", false),
    ];
    let cells = ["api", "3 (5m ago)", "<none>", "12m", "Running", "40s"].map(String::from);
    let (created, received) = (Timestamp::new(1_000), Timestamp::new(1_040));
    let aging = |cell, since, said, lead: Option<&str>| Aging {
        cell,
        since,
        said,
        lead: lead.map(String::from),
        turn: Timestamp::default(),
    };
    assert_eq!(
        agings(&columns, &cells, Some(created), received),
        [
            aging(1, received, 300, Some("3")),
            aging(3, received, 720, None),
            aging(5, created, 0, None),
        ]
    );
}

#[test]
fn an_aging_cell_reads_at_now_and_says_when_it_next_turns() {
    let age = Aging {
        cell: 0,
        since: Timestamp::new(1_000),
        said: 0,
        lead: None,
        turn: Timestamp::default(),
    };
    let now = Timestamp::new(1_179);
    assert_eq!(age.at(now), ("2m59s".into(), Timestamp::new(1_180)));
    let later = Timestamp::new(1_000 + 3_630);
    assert_eq!(age.at(later), ("60m".into(), Timestamp::new(1_000 + 3_660)));
    let restarts = Aging {
        said: 300,
        lead: Some("3".into()),
        ..age
    };
    assert_eq!(restarts.at(now).0, "3 (7m59s ago)");
}
