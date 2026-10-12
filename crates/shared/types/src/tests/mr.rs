//! An MR: the worst run of its CI, and the times its reviewers set.

use crate::{CiState, MrState, Timestamp};

use super::attention::DAY;

#[test]
fn the_worst_check_of_a_run_is_the_one_it_reports() {
    use CiState::{Canceled, Failed, Pending, Running, Skipped, Success, Unknown};
    assert_eq!(Success.worst(Failed), Failed);
    assert_eq!(Failed.worst(Running), Failed);
    assert_eq!(Running.worst(Success), Running);
    assert_eq!(Pending.worst(Canceled), Pending, "the run is not done");
    assert_eq!(Skipped.worst(Success), Success, "a run that passed passed");
    assert_eq!(Unknown.worst(Success), Unknown);
}

/// One reviewer's verdict at a time.
fn reviewer(name: &str, state: crate::ReviewState, at: i64) -> crate::Reviewer {
    crate::Reviewer {
        name: name.into(),
        state,
        at: Some(Timestamp::new(at)),
    }
}

fn details(reviewers: Vec<crate::Reviewer>, approved: bool) -> crate::MrDetails {
    crate::MrDetails {
        title: String::new(),
        description: String::new(),
        author: "haoov".into(),
        source_branch: "one".into(),
        target_branch: "main".into(),
        state: MrState::Open,
        draft: false,
        created_at: Timestamp::new(0),
        updated_at: Timestamp::new(0),
        web_url: String::new(),
        approval: Some(crate::MrApproval {
            approved,
            approved_by_me: false,
            approved_by: Vec::new(),
        }),
        reviewers,
    }
}

#[test]
fn the_times_the_rules_rest_on_come_off_the_reviewers() {
    use crate::ReviewState::{Approved, ChangesRequested, Requested};
    let mr = details(
        vec![
            reviewer("one", Requested, 5 * DAY),
            reviewer("two", Requested, 3 * DAY),
            reviewer("three", ChangesRequested, 7 * DAY),
            reviewer("four", ChangesRequested, 9 * DAY),
        ],
        false,
    );
    assert_eq!(
        mr.review_requested_at(),
        Some(Timestamp::new(3 * DAY)),
        "waiting since the first was asked"
    );
    assert_eq!(
        mr.changes_requested_at(),
        Some(Timestamp::new(7 * DAY)),
        "and asked for changes since the first said so"
    );
    assert!(mr.changes_requested());
    assert_eq!(mr.approved_at(), None);

    let approved = details(
        vec![
            reviewer("one", Approved, 4 * DAY),
            reviewer("two", Approved, 8 * DAY),
        ],
        true,
    );
    assert_eq!(
        approved.approved_at(),
        Some(Timestamp::new(8 * DAY)),
        "approved when the last one approved"
    );
    let short = details(vec![reviewer("one", Approved, 4 * DAY)], false);
    assert_eq!(short.approved_at(), None, "one approval was not enough");
}
