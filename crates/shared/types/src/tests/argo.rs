//! What an Argo CD application says of itself: the revision it synced to and how it syncs.

use crate::{short_sha, sync_policy};

#[test]
fn a_commit_is_cut_to_seven_and_anything_else_stays_whole() {
    let sha = "0123456789abcdef0123456789abcdef01234567";
    assert_eq!(short_sha(sha), "0123456");
    assert_eq!(short_sha(&sha[..39]), &sha[..39], "39 characters");
    let longer = format!("{sha}8");
    assert_eq!(short_sha(&longer), longer, "41 characters");
    let named = "0123456789abcdef0123456789abcdef0123456g";
    assert_eq!(short_sha(named), named, "not all hex");
}

#[test]
fn a_sync_policy_says_manual_or_auto_with_what_it_does() {
    assert_eq!(sync_policy(None), "manual");
    assert_eq!(sync_policy(Some((false, false))), "auto");
    assert_eq!(sync_policy(Some((true, false))), "auto · prune");
    assert_eq!(sync_policy(Some((false, true))), "auto · heal");
    assert_eq!(sync_policy(Some((true, true))), "auto · prune · heal");
}
