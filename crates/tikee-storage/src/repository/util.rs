use std::sync::{OnceLock, RwLock};

use time::{Duration, OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};
use uuid::Uuid;

static TIMESTAMP_OFFSET: OnceLock<RwLock<UtcOffset>> = OnceLock::new();

pub fn set_timestamp_offset(offset: UtcOffset) {
    let lock = TIMESTAMP_OFFSET.get_or_init(|| RwLock::new(UtcOffset::UTC));
    if let Ok(mut guard) = lock.write() {
        *guard = offset;
    }
}

pub fn parse_timestamp_offset(value: &str) -> Result<UtcOffset, time::error::ComponentRange> {
    if value.eq_ignore_ascii_case("utc") || value == "Z" || value == "+00:00" {
        return Ok(UtcOffset::UTC);
    }
    let sign = if value.starts_with('-') { -1 } else { 1 };
    let trimmed = value.trim_start_matches(['+', '-']);
    let mut parts = trimmed.split(':');
    let hours = parts.next().unwrap_or("0").parse::<i8>().unwrap_or(0);
    let minutes = parts.next().unwrap_or("0").parse::<i8>().unwrap_or(0);
    UtcOffset::from_hms(sign * hours, sign * minutes, 0)
}

pub(super) fn now_rfc3339() -> String {
    format_rfc3339(OffsetDateTime::now_utc())
}

pub(super) fn rfc3339_after_seconds(seconds: i64) -> String {
    format_rfc3339(OffsetDateTime::now_utc() + Duration::seconds(seconds))
}

fn timestamp_offset() -> UtcOffset {
    TIMESTAMP_OFFSET
        .get_or_init(|| RwLock::new(UtcOffset::UTC))
        .read()
        .map_or(UtcOffset::UTC, |guard| *guard)
}

fn format_rfc3339(value: OffsetDateTime) -> String {
    value
        .to_offset(timestamp_offset())
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

pub(super) fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::now_v7().simple())
}
