use std::time::Duration;

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;

/// How long ago, in as few characters as it takes: `now`, `2m`, `6h`, `5d`.
pub fn ago(age: Duration) -> String {
    let seconds = age.as_secs();
    if seconds < MINUTE {
        return "now".to_string();
    }
    if seconds < HOUR {
        return format!("{}m", seconds / MINUTE);
    }
    if seconds < DAY {
        return format!("{}h", seconds / HOUR);
    }
    format!("{}d", seconds / DAY)
}
