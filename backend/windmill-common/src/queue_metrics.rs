//! The queue metrics the monitor samples into `metrics` (`queue_count_{tag}` and
//! `queue_delay_{tag}`), and how a stored series is drawn back.

use std::collections::BTreeMap;

use serde::Serialize;
use sqlx::{Pool, Postgres};

pub const QUEUE_COUNT_PREFIX: &str = "queue_count_";
pub const QUEUE_DELAY_PREFIX: &str = "queue_delay_";

/// A backlogged tag whose value has not moved is re-sampled only this often. A longer heartbeat
/// writes fewer rows, but keeps a tag whose drain was never recorded (no server was up when it
/// drained) drawn as backlogged for longer.
pub const QUEUE_METRIC_HEARTBEAT_SECS: f64 = 5.0 * 60.0;

/// A series silent for longer than this has drained: the sampler stops looking for it, so no
/// closing zero will come, and it is drawn as zero from there. Heartbeats land up to a monitor
/// tick and a sampling slot late, so this must stay well above their real spacing.
pub const QUEUE_METRIC_STALE_SECS: f64 = 3.0 * QUEUE_METRIC_HEARTBEAT_SECS;

/// Slots a series is split into, whatever the window. A slot draws at most four vertices, so a
/// line stays near 500 points however many rows the window holds.
const QUEUE_METRICS_SERIES_SLOTS: f64 = 120.0;

#[derive(Serialize)]
pub struct QueueMetricsSeries {
    /// The window drawn, in epoch milliseconds.
    pub from: i64,
    pub to: i64,
    pub tags: Vec<QueueTagSeries>,
}

#[derive(Serialize)]
pub struct QueueTagSeries {
    pub tag: String,
    /// Vertices `[epoch ms, value]` of a line joined by straight segments.
    pub count: Vec<(i64, f64)>,
    pub delay: Vec<(i64, f64)>,
}

/// The queue metrics of the last `window_secs`, each series aggregated per slot by the database
/// and drawn by [`render_series`], so the size is bounded by the number of tags rather than by
/// how many rows they wrote.
///
/// Reads the metrics of every workspace's tags: a caller exposing the result MUST restrict it to
/// devops users, as `GET /workers/queue_metrics_series` does.
pub async fn read_queue_metrics_series(
    db: &Pool<Postgres>,
    window_secs: f64,
) -> crate::error::Result<QueueMetricsSeries> {
    let to = sqlx::query_scalar!("SELECT EXTRACT(EPOCH FROM now())::double precision AS \"now!\"")
        .fetch_one(db)
        .await?;
    let from = to - window_secs;

    // Slot -1 holds the samples written before the window, of which only the last is used: it
    // sets the value in force at the left edge. A series silent for longer than the stale window
    // reads as zero, so nothing older can matter. Arrays compare element by element, so
    // `max(ARRAY[t, v])` is the slot's latest sample, found without sorting every row.
    let rows = sqlx::query!(
        "SELECT id AS \"id!\", slot AS \"slot!\", min(t) AS \"first!\", max(t) AS \"last!\",
            max(v) AS \"peak!\", (max(ARRAY[t, v]))[2] AS \"last_value!\"
        FROM (
            SELECT id, EXTRACT(EPOCH FROM created_at)::double precision AS t,
                CASE WHEN jsonb_typeof(value) = 'number' THEN value::double precision END AS v,
                greatest(floor(
                    (EXTRACT(EPOCH FROM created_at)::double precision - $1::double precision)
                        / $2::double precision
                ), -1)::int AS slot
            FROM metrics
            WHERE id LIKE 'queue_%'
                AND created_at > to_timestamp($1::double precision - $3::double precision)
        ) s
        WHERE v IS NOT NULL
        GROUP BY id, slot
        ORDER BY id, slot",
        from,
        window_secs / QUEUE_METRICS_SERIES_SLOTS,
        QUEUE_METRIC_STALE_SECS,
    )
    .fetch_all(db)
    .await?;

    #[derive(Default)]
    struct Stored {
        carried: Option<(f64, f64)>,
        slots: Vec<MetricSlot>,
    }
    // [count, delay] per tag.
    let mut stored: BTreeMap<String, [Stored; 2]> = BTreeMap::new();
    for row in rows {
        let (series, tag) = if let Some(tag) = row.id.strip_prefix(QUEUE_COUNT_PREFIX) {
            (0, tag)
        } else if let Some(tag) = row.id.strip_prefix(QUEUE_DELAY_PREFIX) {
            (1, tag)
        } else {
            continue;
        };
        let series = &mut stored.entry(tag.to_string()).or_default()[series];
        if row.slot < 0 {
            series.carried = Some((row.last, row.last_value));
        } else {
            series.slots.push(MetricSlot {
                first: row.first,
                last: row.last,
                peak: row.peak,
                last_value: row.last_value,
            });
        }
    }

    let tags = stored
        .into_iter()
        .map(|(tag, [count, delay])| QueueTagSeries {
            tag,
            count: render_series(count.carried, &count.slots, from, to),
            delay: render_series(delay.carried, &delay.slots, from, to),
        })
        // A tag that drained before the window has nothing to draw in it.
        .filter(|s| s.count.iter().chain(&s.delay).any(|(_, v)| *v != 0.0))
        .collect();

    Ok(QueueMetricsSeries {
        from: (from * 1000.0).round() as i64,
        to: (to * 1000.0).round() as i64,
        tags,
    })
}

/// The stored samples of one series that fall in one time slot.
#[derive(Debug, Clone, Copy)]
pub struct MetricSlot {
    /// When the first and the last sample of the slot were written, in epoch seconds.
    pub first: f64,
    pub last: f64,
    pub peak: f64,
    /// The value of the last sample, which holds until the next one.
    pub last_value: f64,
}

/// Draw a stored series over `[from, to]` (epoch seconds) as the vertices of a line joined by
/// straight segments, each `(epoch ms, value)`.
///
/// A sample holds its value until the next one, or until the series has been silent for
/// [`QUEUE_METRIC_STALE_SECS`]. `carried` is the last sample before `from` (when it was written,
/// and its value), which sets the left edge. A slot draws its peak across the span of its samples,
/// so a spike shorter than a slot still shows at full height.
pub fn render_series(
    carried: Option<(f64, f64)>,
    slots: &[MetricSlot],
    from: f64,
    to: f64,
) -> Vec<(i64, f64)> {
    let mut line = Line::default();
    let mut held = carried.filter(|(at, _)| from - at <= QUEUE_METRIC_STALE_SECS);
    if let Some((_, value)) = held {
        line.push(from, value);
    }
    for slot in slots {
        let entering = hold(&mut line, &mut held, slot.first);
        line.push(slot.first, entering);
        line.push(slot.first, slot.peak);
        line.push(slot.last, slot.peak);
        line.push(slot.last, slot.last_value);
        held = Some((slot.last, slot.last_value));
    }
    if !line.0.is_empty() {
        let value = hold(&mut line, &mut held, to);
        line.push(to, value);
    }
    line.0
}

/// The value `held` still has at `t`. When the series went silent for too long before `t`, draws
/// its drop to zero and forgets it.
fn hold(line: &mut Line, held: &mut Option<(f64, f64)>, t: f64) -> f64 {
    let Some((at, value)) = *held else {
        return 0.0;
    };
    let stale_at = at + QUEUE_METRIC_STALE_SECS;
    if t <= stale_at {
        return value;
    }
    line.push(stale_at, value);
    line.push(stale_at, 0.0);
    *held = None;
    0.0
}

#[derive(Default)]
struct Line(Vec<(i64, f64)>);

impl Line {
    fn push(&mut self, t: f64, value: f64) {
        let point = ((t * 1000.0).round() as i64, value);
        match self.0.as_mut_slice() {
            [.., last] if *last == point => {}
            // A horizontal run only needs its two ends.
            [.., a, b] if a.1 == value && b.1 == value => b.0 = point.0,
            _ => self.0.push(point),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FROM: f64 = 1_000_000.0;
    const TO: f64 = FROM + 3600.0;

    fn slot(first: f64, last: f64, peak: f64, last_value: f64) -> MetricSlot {
        MetricSlot { first: FROM + first, last: FROM + last, peak, last_value }
    }

    fn at(secs: f64, value: f64) -> (i64, f64) {
        (((FROM + secs) * 1000.0) as i64, value)
    }

    #[test]
    fn a_value_holds_until_the_next_sample_and_a_drain_drops_where_it_was_written() {
        let line = render_series(
            None,
            &[
                slot(60.0, 60.0, 3.0, 3.0),
                slot(600.0, 600.0, 2.0, 2.0),
                slot(900.0, 900.0, 0.0, 0.0),
            ],
            FROM,
            TO,
        );
        assert_eq!(
            line,
            vec![
                at(60.0, 0.0),
                at(60.0, 3.0),
                at(600.0, 3.0),
                at(600.0, 2.0),
                at(900.0, 2.0),
                at(900.0, 0.0),
                at(3600.0, 0.0),
            ]
        );
    }

    #[test]
    fn a_series_silent_past_the_stale_window_drops_to_zero() {
        let line = render_series(None, &[slot(60.0, 60.0, 3.0, 3.0)], FROM, TO);
        let dropped = 60.0 + QUEUE_METRIC_STALE_SECS;
        assert_eq!(
            line,
            vec![
                at(60.0, 0.0),
                at(60.0, 3.0),
                at(dropped, 3.0),
                at(dropped, 0.0),
                at(3600.0, 0.0)
            ]
        );
    }

    #[test]
    fn a_slot_draws_its_peak_then_continues_from_its_last_sample() {
        // Rows at 60 (5), 70 (9), 80 (4) collapsed into one slot.
        let line = render_series(
            Some((FROM - 30.0, 2.0)),
            &[slot(60.0, 80.0, 9.0, 4.0)],
            FROM,
            FROM + 120.0,
        );
        assert_eq!(
            line,
            vec![
                at(0.0, 2.0),
                at(60.0, 2.0),
                at(60.0, 9.0),
                at(80.0, 9.0),
                at(80.0, 4.0),
                at(120.0, 4.0)
            ]
        );
    }
}
