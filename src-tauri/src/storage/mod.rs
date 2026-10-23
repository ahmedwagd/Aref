//! Persistence: the database file lifecycle, the schema, and the repositories.
//!
//! `storage` is infrastructure (design.md §3.2 rule 1). It imports nothing from
//! `providers` or `commands`; the record types below are the ones those layers consume,
//! and they live here because the schema defines them (SRS §5.1–§5.3).
//!
//! The only key-derived column in this database is `provider_keys.masked_preview`.
//! No table holds key material, and none may gain such a column (`design.md` §8.2).

use std::time::{SystemTime, UNIX_EPOCH};

pub mod db;
pub mod providers_repo;
pub mod settings_repo;

/// Why a database operation failed.
///
/// Deliberately carries no text from the value in question: a message here reaches an
/// `AppError::Internal` and never a log line with content (`AGENTS.md` §3.5).
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("the database could not be opened or queried")]
    Database(#[from] sqlx::Error),
    #[error("the migration could not be prepared or applied")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("the database file could not be copied or removed")]
    File(#[from] std::io::Error),
}

impl StorageError {
    /// The `AppError` branch a repository failure maps to (design.md §4.6).
    ///
    /// The §7 catalog has no storage code, so an unexpected database failure is
    /// `E_INTERNAL` — the row that exists for exactly this case.
    pub fn into_app_error(self) -> crate::error::AppError {
        crate::error::AppError::internal()
    }
}

/// The RFC 3339 UTC timestamp an `updated_at` column holds (SRS §5.1).
///
/// Written by hand rather than pulled from a date crate: the schema fixes the format,
/// not the library, and this is the only place the app formats a time. Sub-second
/// precision is dropped, which keeps the column sortable as text.
pub fn rfc3339_utc(at: SystemTime) -> String {
    let seconds = match at.duration_since(UNIX_EPOCH) {
        Ok(elapsed) => elapsed.as_secs() as i64,
        // A clock before 1970 is possible on a misconfigured machine. Falling back to
        // the epoch keeps the column well-formed instead of panicking on user input
        // that never arrives (`AGENTS.md` §9: no panic).
        Err(_) => 0,
    };
    let days = seconds.div_euclid(86_400);
    let second_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);

    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        second_of_day / 3_600,
        (second_of_day % 3_600) / 60,
        second_of_day % 60
    )
}

/// The current instant in the same form.
pub fn timestamp_now() -> String {
    rfc3339_utc(SystemTime::now())
}

/// Days since the Unix epoch → `(year, month, day)` in the proleptic Gregorian calendar.
///
/// Howard Hinnant's `civil_from_days`, which is exact for the whole `i64` range and
/// needs no lookup tables.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = if month_part < 10 {
        month_part + 3
    } else {
        month_part - 9
    };
    (year + i64::from(month <= 2), month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn the_epoch_is_formatted_as_rfc3339_utc() {
        assert_eq!(rfc3339_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn a_known_instant_matches_the_reference_value() {
        let at = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        assert_eq!(rfc3339_utc(at), "2023-11-14T22:13:20Z");
    }

    #[test]
    fn a_leap_day_and_a_year_boundary_are_not_off_by_one() {
        assert_eq!(
            rfc3339_utc(UNIX_EPOCH + Duration::from_secs(1_709_164_800)),
            "2024-02-29T00:00:00Z"
        );
        assert_eq!(
            rfc3339_utc(UNIX_EPOCH + Duration::from_secs(1_735_689_599)),
            "2024-12-31T23:59:59Z"
        );
    }

    #[test]
    fn a_clock_before_the_epoch_does_not_panic() {
        let at = UNIX_EPOCH - Duration::from_secs(60);
        assert_eq!(rfc3339_utc(at), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn a_repository_failure_maps_to_the_internal_code() {
        let error = StorageError::Database(sqlx::Error::RowNotFound);
        assert_eq!(error.into_app_error().code(), "E_INTERNAL");
    }
}
