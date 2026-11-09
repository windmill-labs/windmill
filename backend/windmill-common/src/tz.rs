//! An IANA time zone as a `chrono::TimeZone`, resolved from the newer of two copies of
//! the time zone database: the system's (`$TZDIR`, else `/usr/share/zoneinfo`) and the
//! one bundled into the binary; a name missing from that copy is looked up in the other.
//! chrono-tz compiles its rules in and is no longer updated, so zones that change their
//! rules (British Columbia, Manitoba, ...) kept their old offsets.

use std::{
    fmt,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    str::FromStr,
    sync::LazyLock,
};

use chrono::{FixedOffset, LocalResult, NaiveDate, NaiveDateTime, NaiveTime};
use jiff::tz::{AmbiguousOffset, TimeZone, TimeZoneDatabase};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tz(TimeZone);

impl Tz {
    pub const UTC: Tz = Tz(TimeZone::UTC);

    fn offset(&self, offset: jiff::tz::Offset, at: jiff::Timestamp) -> TzOffset {
        TzOffset {
            tz: self.clone(),
            // The database holds no offset beyond chrono's ±24h.
            fixed: FixedOffset::east_opt(offset.seconds())
                .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap()),
            at,
        }
    }
}

impl FromStr for Tz {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        static DATABASES: LazyLock<Databases> = LazyLock::new(|| {
            Databases::choose(
                system_database(),
                TimeZoneDatabase::bundled(),
                jiff_tzdb::VERSION,
            )
        });
        DATABASES.lookup(name)
    }
}

struct Databases {
    primary: TimeZoneDatabase,
    /// Debian (trixie onward) ships the backward-compatible names (`US/Eastern`,
    /// `Asia/Calcutta`, which browsers still report) in `tzdata-legacy`, which images
    /// rarely install, so the system copy can lack a name the bundled one has.
    fallback: TimeZoneDatabase,
}

impl Databases {
    /// A system copy of unknown version wins: it is what the host, or `$TZDIR`, asked for.
    fn choose(
        system: Option<(TimeZoneDatabase, Option<String>)>,
        bundled: TimeZoneDatabase,
        bundled_version: Option<&str>,
    ) -> Databases {
        let Some((system, system_version)) = system else {
            return Databases { primary: bundled, fallback: TimeZoneDatabase::none() };
        };
        // IANA versions are a year then a letter (`2026f`), so they order as strings.
        let bundled_is_newer = matches!(
            (bundled_version, system_version.as_deref()),
            (Some(b), Some(s)) if b > s
        );
        tracing::info!(
            "time zones resolve from the {} tzdb (system {}, bundled {})",
            if bundled_is_newer {
                "bundled"
            } else {
                "system"
            },
            system_version.as_deref().unwrap_or("of unknown version"),
            bundled_version.unwrap_or("of unknown version"),
        );
        if bundled_is_newer {
            Databases { primary: bundled, fallback: system }
        } else {
            Databases { primary: system, fallback: bundled }
        }
    }

    fn lookup(&self, name: &str) -> Result<Tz, String> {
        self.primary
            .get(name)
            .or_else(|_| self.fallback.get(name))
            .map(Tz)
            .map_err(|_| format!("'{name}' is not a valid timezone"))
    }
}

/// The directories jiff searches, in its order, so the version read here is that of the
/// copy whose zones are read.
fn system_database() -> Option<(TimeZoneDatabase, Option<String>)> {
    let dirs: Vec<PathBuf> = match std::env::var_os("TZDIR") {
        Some(dir) => vec![dir.into()],
        None => [
            "/usr/share/zoneinfo",
            "/usr/share/lib/zoneinfo",
            "/etc/zoneinfo",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect(),
    };
    dirs.into_iter().find_map(|dir| {
        let db = TimeZoneDatabase::from_dir(&dir).ok()?;
        (!db.is_definitively_empty()).then(|| (db, system_version(&dir)))
    })
}

/// The `# version 2026f` line heading `tzdata.zi`, or macOS's `+VERSION`.
fn system_version(dir: &Path) -> Option<String> {
    let from_zi = std::fs::File::open(dir.join("tzdata.zi"))
        .ok()
        .and_then(|f| {
            let mut line = String::new();
            BufReader::new(f).read_line(&mut line).ok()?;
            Some(line.strip_prefix("# version ")?.trim().to_string())
        });
    from_zi.or_else(|| {
        let v = std::fs::read_to_string(dir.join("+VERSION")).ok()?;
        Some(v.trim().to_string()).filter(|v| !v.is_empty())
    })
}

/// The offset of a `chrono::DateTime<Tz>`. chrono rebuilds the zone from it
/// (`TimeZone::from_offset`), so it carries the zone, and the instant it applies at so
/// that `%Z` prints the zone's abbreviation as chrono-tz did.
#[derive(Clone, Debug)]
pub struct TzOffset {
    tz: Tz,
    fixed: FixedOffset,
    at: jiff::Timestamp,
}

impl chrono::Offset for TzOffset {
    fn fix(&self) -> FixedOffset {
        self.fixed
    }
}

impl fmt::Display for TzOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tz.0.to_offset_info(self.at).abbreviation())
    }
}

/// Local times resolve as in chrono-tz: a time in a gap has no offset, and one in a fold
/// yields the earlier instant first.
impl chrono::TimeZone for Tz {
    type Offset = TzOffset;

    fn from_offset(offset: &Self::Offset) -> Self {
        offset.tz.clone()
    }

    fn offset_from_local_date(&self, local: &NaiveDate) -> LocalResult<Self::Offset> {
        // As chrono-tz: a date takes any offset in effect on that day, preferring one
        // that is unambiguous at its first or last second.
        let earliest = self.offset_from_local_datetime(&local.and_time(NaiveTime::MIN));
        let latest = self.offset_from_local_datetime(&local.and_hms_opt(23, 59, 59).unwrap());
        use LocalResult::*;
        match (earliest, latest) {
            (result @ Single(_), _) => result,
            (_, result @ Single(_)) => result,
            (Ambiguous(offset, _), _) => Single(offset),
            (_, Ambiguous(offset, _)) => Single(offset),
            (None, None) => None,
        }
    }

    fn offset_from_local_datetime(&self, local: &NaiveDateTime) -> LocalResult<Self::Offset> {
        let Some(civil) = to_civil(local) else {
            return LocalResult::None;
        };
        let at = |offset: jiff::tz::Offset| {
            self.offset(
                offset,
                offset.to_timestamp(civil).unwrap_or(jiff::Timestamp::MAX),
            )
        };
        match self.0.to_ambiguous_timestamp(civil).offset() {
            AmbiguousOffset::Unambiguous { offset } => LocalResult::Single(at(offset)),
            AmbiguousOffset::Fold { before, after } => {
                LocalResult::Ambiguous(at(before), at(after))
            }
            AmbiguousOffset::Gap { .. } => LocalResult::None,
        }
    }

    fn offset_from_utc_date(&self, utc: &NaiveDate) -> Self::Offset {
        self.offset_from_utc_datetime(&utc.and_time(NaiveTime::MIN))
    }

    fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> Self::Offset {
        let utc = utc.and_utc();
        let at = jiff::Timestamp::new(utc.timestamp(), utc.timestamp_subsec_nanos() as i32)
            .unwrap_or(if utc.timestamp() < 0 {
                jiff::Timestamp::MIN
            } else {
                jiff::Timestamp::MAX
            });
        self.offset(self.0.to_offset(at), at)
    }
}

fn to_civil(local: &NaiveDateTime) -> Option<jiff::civil::DateTime> {
    use chrono::{Datelike, Timelike};
    jiff::civil::DateTime::new(
        i16::try_from(local.year()).ok()?,
        local.month() as i8,
        local.day() as i8,
        local.hour() as i8,
        local.minute() as i8,
        local.second() as i8,
        // chrono carries a leap second as nanoseconds past 1e9, which jiff has no room for.
        (local.nanosecond() % 1_000_000_000) as i32,
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::ScheduleType;
    use chrono::{TimeZone as _, Utc};

    /// The bundled copy, so the expectations do not depend on the host's tzdata.
    fn bundled(name: &str) -> Tz {
        Tz(jiff::tz::TimeZoneDatabase::bundled().get(name).unwrap())
    }

    fn utc(s: &str) -> chrono::DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn schedules_follow_current_tzdata() {
        // British Columbia stopped changing its clocks in 2026: 08:00 stays at UTC-7.
        let vancouver = bundled("America/Vancouver");
        let from = utc("2026-11-03T16:00:00Z").with_timezone(&vancouver);
        for version in ["v1", "v2"] {
            let daily = ScheduleType::from_str("0 0 8 * * *", Some(version), true).unwrap();
            let next = daily.find_next(&from).unwrap();
            assert_eq!(
                next.with_timezone(&Utc),
                utc("2026-11-04T15:00:00Z"),
                "{version}"
            );
        }
    }

    #[test]
    fn the_newer_copy_wins_and_the_other_fills_in_missing_names() {
        // An empty database stands in for a system copy lacking the legacy names.
        let with_system = |version: Option<&str>| {
            Databases::choose(
                Some((TimeZoneDatabase::none(), version.map(str::to_string))),
                TimeZoneDatabase::bundled(),
                Some("2026e"),
            )
        };
        let system_wins = |dbs: &Databases| dbs.primary.is_definitively_empty();
        assert!(!system_wins(&with_system(Some("2024a"))));
        assert!(system_wins(&with_system(Some("2026f"))));
        assert!(system_wins(&with_system(None)));

        let dbs = with_system(Some("2026f"));
        for name in ["Asia/Calcutta", "US/Eastern", "Europe/Kiev", "EST5EDT"] {
            assert!(dbs.lookup(name).is_ok(), "{name}");
        }
        assert!(dbs.lookup("Not/AZone").is_err());
    }

    #[test]
    fn local_times_resolve_as_chrono_tz_did() {
        let ny = bundled("America/New_York");
        let local = |s: &str| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap();

        let LocalResult::Ambiguous(earlier, later) =
            ny.from_local_datetime(&local("2025-11-02 01:30"))
        else {
            panic!("a time in the fall-back hour is ambiguous");
        };
        assert_eq!(earlier.with_timezone(&Utc), utc("2025-11-02T05:30:00Z"));
        assert_eq!(later.with_timezone(&Utc), utc("2025-11-02T06:30:00Z"));
        assert_eq!(earlier.format("%Z %z").to_string(), "EDT -0400");
        assert_eq!(later.format("%Z %z").to_string(), "EST -0500");

        assert!(ny
            .from_local_datetime(&local("2025-03-09 02:30"))
            .single()
            .is_none());
        assert_eq!(
            ny.with_ymd_and_hms(2025, 3, 9, 3, 0, 0)
                .unwrap()
                .with_timezone(&Utc),
            utc("2025-03-09T07:00:00Z")
        );
    }
}
