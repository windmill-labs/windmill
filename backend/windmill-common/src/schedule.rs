/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::error::{Error, Result};
use crate::utils::ScheduleType;

pub use windmill_types::schedule::*;

/// Upper bound on the walk in [`SkipDetail::Count`]. A gap on a per-second cron
/// can hold arbitrarily many occurrences, and the caller only needs enough of a
/// number to act on.
pub const MAX_COUNTED_SKIPS: u32 = 1000;

/// How much work [`reconstruct_occurrences`] should do per gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipDetail {
    /// One `find_next` per gap. Bounded regardless of how far behind the
    /// schedule has fallen, which is what makes this affordable per list page.
    DetectOnly,
    /// Walk the gap to count what was lost, up to [`MAX_COUNTED_SKIPS`].
    Count,
}

/// One completed root occurrence of a schedule, recovered from its job row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconstructedOccurrence {
    /// The `scheduled_for` the push computed for this occurrence.
    pub scheduled_for: DateTime<Utc>,
    /// Whether occurrences were lost between this one and the next. `None` when
    /// the pair cannot be compared: it straddles the baseline, or this is the
    /// oldest occurrence in the window and has no successor to compare against.
    pub skipped_after: Option<bool>,
    /// How many were lost. `None` unless [`SkipDetail::Count`] was asked for and
    /// the pair is comparable.
    pub skipped_count: Option<u32>,
    /// `skipped_count` stopped at [`MAX_COUNTED_SKIPS`] and is a lower bound.
    pub count_capped: bool,
}

/// Recovers the occurrence sequence of a schedule from the `created_at` of the
/// root jobs it pushed, newest first, and reports which gaps lost occurrences.
///
/// `scheduled_for = find_next(created_at)` holds exactly: `push_scheduled_job`
/// anchors on `now_from_db` inside the transaction that inserts the job, and
/// `v2_job.created_at` defaults to that same transaction timestamp. The parse
/// flags here must stay identical to the ones the push uses, or the recovered
/// times drift from the real ones.
///
/// `baseline` is the schedule's `occurrence_baseline_at`. A gap whose older end
/// predates it spans a pause, a cron change, a re-enable or a reconciler re-arm,
/// none of which mean the schedule lost runs, so it is reported as `None` rather
/// than counted. `None` means no baseline is known and every gap is comparable.
///
/// `created_at_newest_first` must contain **every** root occurrence in the
/// window, including ones that completed as `skipped`. A hole in the middle
/// manufactures a phantom gap.
pub fn reconstruct_occurrences(
    schedule: &str,
    cron_version: Option<&str>,
    timezone: &str,
    created_at_newest_first: &[DateTime<Utc>],
    baseline: Option<DateTime<Utc>>,
    detail: SkipDetail,
) -> Result<Vec<ReconstructedOccurrence>> {
    let sched = ScheduleType::from_str(schedule, cron_version, false)?;
    let tz = chrono_tz::Tz::from_str(timezone)
        .map_err(|e| Error::BadRequest(format!("invalid timezone {timezone}: {e}")))?;

    let next_after = |from: DateTime<Utc>| -> DateTime<Utc> {
        sched
            .find_next(&from.with_timezone(&tz))
            .with_timezone(&Utc)
    };

    let scheduled_for: Vec<DateTime<Utc>> = created_at_newest_first
        .iter()
        .map(|created_at| next_after(*created_at))
        .collect();

    let mut out = Vec::with_capacity(scheduled_for.len());
    for (i, &current) in scheduled_for.iter().enumerate() {
        // Newest first, so the occurrence that follows `current` sits at `i - 1`
        // and the gap belongs to the older of the pair.
        let successor = if i == 0 {
            None
        } else {
            Some(scheduled_for[i - 1])
        };
        let comparable = successor.filter(|&successor| {
            successor > current && baseline.is_none_or(|baseline| current >= baseline)
        });

        let Some(successor) = comparable else {
            out.push(ReconstructedOccurrence {
                scheduled_for: current,
                skipped_after: None,
                skipped_count: None,
                count_capped: false,
            });
            continue;
        };

        let predicted = next_after(current);
        if predicted >= successor {
            out.push(ReconstructedOccurrence {
                scheduled_for: current,
                skipped_after: Some(false),
                skipped_count: matches!(detail, SkipDetail::Count).then_some(0),
                count_capped: false,
            });
            continue;
        }

        let (skipped_count, count_capped) = match detail {
            SkipDetail::DetectOnly => (None, false),
            SkipDetail::Count => {
                let mut cursor = predicted;
                let mut n = 1;
                while n < MAX_COUNTED_SKIPS {
                    cursor = next_after(cursor);
                    if cursor >= successor {
                        break;
                    }
                    n += 1;
                }
                (Some(n), n == MAX_COUNTED_SKIPS)
            }
        };

        out.push(ReconstructedOccurrence {
            scheduled_for: current,
            skipped_after: Some(true),
            skipped_count,
            count_capped,
        });
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVERY_10S: &str = "*/10 * * * * *";

    fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn run(
        cron: &str,
        created_at_newest_first: &[DateTime<Utc>],
        baseline: Option<DateTime<Utc>>,
        detail: SkipDetail,
    ) -> Vec<ReconstructedOccurrence> {
        reconstruct_occurrences(
            cron,
            Some("v2"),
            "UTC",
            created_at_newest_first,
            baseline,
            detail,
        )
        .unwrap()
    }

    /// The measured case: a 25 s job on a 10 s cron, which runs every 30 s.
    #[test]
    fn counts_the_occurrences_an_overrun_loses() {
        let got = run(
            EVERY_10S,
            &[
                at("2026-01-01T08:21:15Z"),
                at("2026-01-01T08:20:45Z"),
                at("2026-01-01T08:20:15Z"),
            ],
            None,
            SkipDetail::Count,
        );

        assert_eq!(
            got.iter().map(|o| o.scheduled_for).collect::<Vec<_>>(),
            vec![
                at("2026-01-01T08:21:20Z"),
                at("2026-01-01T08:20:50Z"),
                at("2026-01-01T08:20:20Z"),
            ],
            "scheduled_for is find_next(created_at)"
        );
        // The newest occurrence has no successor yet, so its gap is unknown.
        assert_eq!(got[0].skipped_after, None);
        assert_eq!(got[1].skipped_count, Some(2));
        assert_eq!(got[2].skipped_count, Some(2));
        assert!(got.iter().all(|o| !o.count_capped));
    }

    /// A hole in the input manufactures a gap that never happened, which is why
    /// the query behind this must not filter out `skipped` occurrences.
    #[test]
    fn a_missing_occurrence_manufactures_a_phantom_skip() {
        let full = [
            at("2026-01-01T08:20:21Z"),
            at("2026-01-01T08:20:11Z"),
            at("2026-01-01T08:20:01Z"),
        ];
        let healthy = run(EVERY_10S, &full, None, SkipDetail::Count);
        assert_eq!(healthy[1].skipped_after, Some(false));
        assert_eq!(healthy[2].skipped_after, Some(false));

        let with_hole = [full[0], full[2]];
        let holed = run(EVERY_10S, &with_hole, None, SkipDetail::Count);
        assert_eq!(holed[1].skipped_count, Some(1));
    }

    /// A gap older than the baseline spans a pause, a cron change or a re-enable,
    /// and says nothing about whether the schedule is losing runs.
    #[test]
    fn a_gap_older_than_the_baseline_is_not_comparable() {
        let created_at = [
            at("2026-01-01T09:00:05Z"),
            at("2026-01-01T08:20:15Z"),
            at("2026-01-01T08:20:05Z"),
        ];

        let without_baseline = run(EVERY_10S, &created_at, None, SkipDetail::Count);
        assert!(without_baseline[1].skipped_count.unwrap() > 200);

        // Written when the pause was set, between the two occurrences.
        let with_baseline = run(
            EVERY_10S,
            &created_at,
            Some(at("2026-01-01T08:30:00Z")),
            SkipDetail::Count,
        );
        assert_eq!(with_baseline[1].skipped_after, None);
        assert_eq!(with_baseline[2].skipped_after, None);
    }

    #[test]
    fn a_long_gap_saturates_rather_than_walking_forever() {
        let got = run(
            "* * * * * *",
            &[at("2026-01-02T08:00:00Z"), at("2026-01-01T08:00:00Z")],
            None,
            SkipDetail::Count,
        );
        assert_eq!(got[1].skipped_count, Some(MAX_COUNTED_SKIPS));
        assert!(got[1].count_capped);
    }

    /// The list surface answers the cheap question and never walks.
    #[test]
    fn detect_only_reports_the_gap_without_counting_it() {
        let got = run(
            "* * * * * *",
            &[at("2026-01-02T08:00:00Z"), at("2026-01-01T08:00:00Z")],
            None,
            SkipDetail::DetectOnly,
        );
        assert_eq!(got[1].skipped_after, Some(true));
        assert_eq!(got[1].skipped_count, None);
        assert!(!got[1].count_capped);
    }
}
