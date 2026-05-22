use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

pub(super) fn now_rfc3339() -> String {
    format_rfc3339(OffsetDateTime::now_utc())
}

pub(super) fn rfc3339_after_seconds(seconds: i64) -> String {
    format_rfc3339(OffsetDateTime::now_utc() + Duration::seconds(seconds))
}

fn format_rfc3339(value: OffsetDateTime) -> String {
    value
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

pub(super) fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::now_v7().simple())
}
