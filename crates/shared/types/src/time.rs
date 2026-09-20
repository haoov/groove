use std::fmt;
use std::time::Duration;

use crate::{Error, Result};

const SECONDS_PER_DAY: i64 = 86_400;

/// Unix seconds.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
pub struct Timestamp(i64);

impl Timestamp {
    pub const fn new(seconds: i64) -> Self {
        Self(seconds)
    }

    pub const fn seconds(self) -> i64 {
        self.0
    }

    pub fn now() -> Self {
        let since_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        Self(since_epoch.as_secs() as i64)
    }

    /// How long ago this was at `now`; zero when it is in the future.
    pub fn age_at(self, now: Timestamp) -> Duration {
        Duration::from_secs((now.0 - self.0).max(0) as u64)
    }

    pub fn days_before(self, now: Timestamp) -> i64 {
        (now.0 - self.0).div_euclid(SECONDS_PER_DAY)
    }

    /// The UTC calendar day this falls on.
    pub fn day(self) -> Day {
        Day::from_days(self.0.div_euclid(SECONDS_PER_DAY))
    }

    /// `YYYY-MM-DDTHH:MM:SS`, with `Z`, an offset or neither.
    pub fn parse(text: &str) -> Result<Self> {
        let invalid = || Error::invalid(format!("not a time: {text}"));
        let (date, clock) = text.split_once('T').ok_or_else(invalid)?;
        let (clock, offset) = without_zone(clock);
        let into_day = seconds_of(clock).ok_or_else(invalid)?;
        Ok(Self(
            Day::parse(date)?.days() * SECONDS_PER_DAY + into_day - offset,
        ))
    }
}

/// The clock, and the zone after it in seconds east of UTC.
fn without_zone(clock: &str) -> (&str, i64) {
    if let Some(rest) = clock.strip_suffix('Z') {
        return (rest, 0);
    }
    match clock.rfind(['+', '-']) {
        Some(at) => (&clock[..at], zone(&clock[at..])),
        None => (clock, 0),
    }
}

/// `+HH:MM` or `-HH:MM` as seconds east of UTC.
fn zone(text: &str) -> i64 {
    let sign = match text.starts_with('-') {
        true => -1,
        false => 1,
    };
    let mut parts = text.trim_start_matches(['+', '-']).splitn(2, ':');
    let hours: i64 = parts
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or_default();
    let minutes: i64 = parts
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or_default();
    sign * (hours * 3600 + minutes * 60)
}

/// `HH:MM:SS`, with or without a fraction, as seconds into the day.
fn seconds_of(clock: &str) -> Option<i64> {
    let mut parts = clock.splitn(3, ':');
    let hours: i64 = parts.next()?.parse().ok()?;
    let minutes: i64 = parts.next()?.parse().ok()?;
    let seconds: i64 = match parts.next() {
        Some(text) => text.split('.').next()?.parse().ok()?,
        None => 0,
    };
    let sane = hours < 24 && minutes < 60 && seconds < 61;
    sane.then_some(hours * 3600 + minutes * 60 + seconds)
}

/// A calendar day, `YYYY-MM-DD`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Day {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Day {
    pub fn parse(text: &str) -> Result<Self> {
        let invalid = || Error::invalid(format!("not a day: {text}"));
        let mut parts = text.splitn(3, '-');
        let year = parts
            .next()
            .and_then(|p| p.parse().ok())
            .ok_or_else(invalid)?;
        let month = parts
            .next()
            .and_then(|p| p.parse().ok())
            .ok_or_else(invalid)?;
        let day = parts
            .next()
            .and_then(|p| p.parse().ok())
            .ok_or_else(invalid)?;
        let parsed = Self { year, month, day };
        let valid = (1..=12).contains(&month)
            && (1..=31).contains(&day)
            && Self::from_days(parsed.days()) == parsed;
        valid.then_some(parsed).ok_or_else(invalid)
    }

    /// Days since 1970-01-01.
    pub fn days(self) -> i64 {
        let (y, m, d) = (
            i64::from(self.year),
            i64::from(self.month),
            i64::from(self.day),
        );
        let y = if m <= 2 { y - 1 } else { y };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    pub fn from_days(days: i64) -> Self {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = yoe + era * 400 + i64::from(month <= 2);
        Self {
            year: year as i32,
            month: month as u8,
            day: day as u8,
        }
    }

    pub fn plus_days(self, n: i64) -> Self {
        Self::from_days(self.days() + n)
    }

    /// Signed days from `self` to `other`.
    pub fn days_until(self, other: Day) -> i64 {
        other.days() - self.days()
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl serde::Serialize for Day {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for Day {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        Day::parse(&text).map_err(serde::de::Error::custom)
    }
}
