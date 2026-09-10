//! The queue metrics the monitor samples into `metrics` (`queue_count_{tag}` and
//! `queue_delay_{tag}`), and how a stored series is drawn back.
//!
//! A stored value is a number, held until the next sample, or, for a delay, `{"since": <epoch
//! seconds>}`: the job at the head of the queue has been waiting since then and was still there
//! when sampled, so the delay climbs one second per second until the next sample. Besides
//! [`QueueSample`], the SQL in [`read_queue_metrics_series`] and in `GET /workers/queue_metrics`
//! decodes both shapes.

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

/// Heads that started waiting within this of each other are one wait: jobs queued together
/// leave the head one after another without the delay dropping.
pub const QUEUE_DELAY_SAME_HEAD_SECS: f64 = 1.0;

/// Slots a series is split into, whatever the window. A slot draws at most four vertices, so a
/// line stays near 500 points however many rows the window holds.
const QUEUE_METRICS_SERIES_SLOTS: f64 = 120.0;

/// A stored sample, as it is drawn from the moment it was written until the next one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QueueSample {
    /// A count, or a delay while the head keeps changing, which hovers.
    Held(f64),
    /// A delay while the job that started waiting at `since` (epoch seconds) stays at the head.
    Climbing { since: f64 },
}

impl QueueSample {
    pub fn parse(value: &serde_json::Value) -> Option<Self> {
        match value.get("since") {
            Some(since) => since.as_f64().map(|since| Self::Climbing { since }),
            None => value.as_f64().map(Self::Held),
        }
    }

    pub fn to_json(self) -> serde_json::Value {
        match self {
            Self::Held(value) => serde_json::json!(value),
            Self::Climbing { since } => serde_json::json!({ "since": since }),
        }
    }

    /// Its value at `t`, in epoch seconds.
    pub fn value_at(self, t: f64) -> f64 {
        match self {
            Self::Held(value) => value,
            Self::Climbing { since } => t - since,
        }
    }

    /// When the job at the head of a delay sample written at `at` started waiting.
    pub fn head_since(self, at: f64) -> f64 {
        match self {
            Self::Held(delay) => at - delay,
            Self::Climbing { since } => since,
        }
    }
}

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
    let slot_secs = window_secs / QUEUE_METRICS_SERIES_SLOTS;

    // Slot -1 holds the samples written before the window, of which only the last is used: it
    // sets the value in force at the left edge. A series silent for longer than the stale window
    // reads as zero, so nothing older can matter. Arrays compare element by element, so
    // `max(ARRAY[t, v])` is the slot's latest sample, found without sorting every row. `v` is a
    // sample's value when it was written: for a climbing delay, how long its head had waited.
    //
    // A climb keeps rising until the next sample, so when that sample lands in the same slot
    // (the tag drained, or its head moved), the climb's top is higher than any `v`. Looking the
    // next sample up for the slot's last climb, rather than ordering every row, keeps the pass a
    // plain aggregate; an earlier climb in the same slot still shows up to its last heartbeat.
    let rows = sqlx::query!(
        "WITH slots AS (
            SELECT id, slot, min(t) AS first, max(t) AS last, max(v) AS peak,
                (min(ARRAY[t, v]))[2] AS first_value, (max(ARRAY[t, v]))[2] AS last_value,
                (max(ARRAY[t, climbing]))[2] = 1 AS last_climbing,
                COALESCE(bool_and(climbing = 1) AND max(since) - min(since) < $4, false) AS ramp,
                max(ARRAY[t, since]) FILTER (WHERE climbing = 1) AS last_climb
            FROM (
                SELECT id, t,
                    CASE jsonb_typeof(value)
                        WHEN 'number' THEN value::double precision
                        WHEN 'object' THEN t - (value->>'since')::double precision
                    END AS v,
                    (value->>'since')::double precision AS since,
                    (jsonb_typeof(value) = 'object')::int::double precision AS climbing,
                    greatest(floor((t - $1::double precision) / $2::double precision), -1)::int
                        AS slot
                FROM (
                    SELECT id, value, EXTRACT(EPOCH FROM created_at)::double precision AS t
                    FROM metrics
                    WHERE id LIKE 'queue_%'
                        AND created_at > to_timestamp($1::double precision - $3::double precision)
                ) m
            ) s
            WHERE v IS NOT NULL
            GROUP BY id, slot
        )
        SELECT id AS \"id!\", slot AS \"slot!\", first AS \"first!\", last AS \"last!\",
            greatest(peak, CASE WHEN last_climb[1] < last THEN (
                SELECT EXTRACT(EPOCH FROM min(n.created_at))::double precision
                FROM metrics n
                WHERE n.id = slots.id AND n.id LIKE 'queue_%'
                    AND n.created_at > to_timestamp(last_climb[1] + 0.001)
                    AND n.created_at <= to_timestamp(last + 0.001)
            ) - last_climb[2] END) AS \"peak!\",
            first_value AS \"first_value!\", last_value AS \"last_value!\",
            last_climbing AS \"last_climbing!\", ramp AS \"ramp!\"
        FROM slots
        ORDER BY id, slot",
        from,
        slot_secs,
        QUEUE_METRIC_STALE_SECS,
        QUEUE_DELAY_SAME_HEAD_SECS,
    )
    .fetch_all(db)
    .await?;

    #[derive(Default)]
    struct Stored {
        carried: Option<MetricSlot>,
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
        let slot = MetricSlot {
            first: row.first,
            last: row.last,
            peak: row.peak,
            first_value: row.first_value,
            last_value: row.last_value,
            last_climbing: row.last_climbing,
            ramp: row.ramp,
        };
        if row.slot < 0 {
            series.carried = Some(slot);
        } else {
            series.slots.push(slot);
        }
    }

    let tags = stored
        .into_iter()
        .map(|(tag, [count, delay])| {
            let draw =
                |s: &Stored| render_series(s.carried.as_ref(), &s.slots, from, to, slot_secs);
            QueueTagSeries { count: draw(&count), delay: draw(&delay), tag }
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
    /// The highest value the series drew over the slot, a climb that ends inside it included.
    pub peak: f64,
    pub first_value: f64,
    /// The value of the last sample, which holds (or climbs, for a climbing delay) until the next.
    pub last_value: f64,
    pub last_climbing: bool,
    /// Every sample of the slot climbs from the same head, so the slot is one exact ramp.
    pub ramp: bool,
}

/// Draw a stored series over `[from, to]` (epoch seconds), split into slots of `slot_secs`, as
/// the vertices of a line joined by straight segments, each `(epoch ms, value)`.
///
/// A sample holds its value, or a climbing delay keeps climbing, until the next sample or until
/// the series has been silent for [`QUEUE_METRIC_STALE_SECS`]. `carried` is the slot before
/// `from`, whose last sample sets the left edge. A slot draws its peak across the span of its
/// samples, so a spike shorter than a slot still shows at full height, unless it is a single
/// climb, drawn exactly. A climb gets a vertex at every slot boundary it crosses: the delay axis
/// is logarithmic, so one straight segment across many slots would misplace it.
pub fn render_series(
    carried: Option<&MetricSlot>,
    slots: &[MetricSlot],
    from: f64,
    to: f64,
    slot_secs: f64,
) -> Vec<(i64, f64)> {
    let mut line = Line { points: vec![], from, slot_secs };
    let mut held = carried
        .map(Held::after)
        .filter(|h| from - h.at <= QUEUE_METRIC_STALE_SECS);
    if let Some(h) = held {
        line.push(from, h.value_at(from));
    }
    for slot in slots {
        let entering = line.advance(&mut held, slot.first);
        line.push(slot.first, entering);
        if slot.ramp {
            line.push(slot.first, slot.first_value);
        } else {
            line.push(slot.first, slot.peak);
            line.push(slot.last, slot.peak);
        }
        line.push(slot.last, slot.last_value);
        held = Some(Held::after(slot));
    }
    if !line.points.is_empty() {
        let value = line.advance(&mut held, to);
        line.push(to, value);
    }
    line.points
}

/// The last sample drawn: when it was written, its value then, and whether it climbs from there.
#[derive(Clone, Copy)]
struct Held {
    at: f64,
    value: f64,
    climbing: bool,
}

impl Held {
    fn after(slot: &MetricSlot) -> Self {
        Self { at: slot.last, value: slot.last_value, climbing: slot.last_climbing }
    }

    fn value_at(self, t: f64) -> f64 {
        if self.climbing {
            self.value + (t - self.at)
        } else {
            self.value
        }
    }
}

struct Line {
    points: Vec<(i64, f64)>,
    from: f64,
    slot_secs: f64,
}

impl Line {
    /// The value `held` has at `t`, drawing the climb that leads there and, when the series went
    /// silent for too long first, its drop to zero, after which it is forgotten.
    fn advance(&mut self, held: &mut Option<Held>, t: f64) -> f64 {
        let Some(h) = *held else {
            return 0.0;
        };
        let stale_at = h.at + QUEUE_METRIC_STALE_SECS;
        if h.climbing {
            let end = t.min(stale_at);
            let start = h.at.max(self.from);
            let mut boundary = self.from
                + ((start - self.from) / self.slot_secs).floor() * self.slot_secs
                + self.slot_secs;
            while boundary < end {
                self.push(boundary, h.value_at(boundary));
                boundary += self.slot_secs;
            }
        }
        if t <= stale_at {
            return h.value_at(t);
        }
        self.push(stale_at, h.value_at(stale_at));
        self.push(stale_at, 0.0);
        *held = None;
        0.0
    }

    fn push(&mut self, t: f64, value: f64) {
        let point = ((t * 1000.0).round() as i64, value);
        match self.points.as_mut_slice() {
            [.., last] if *last == point => {}
            // A horizontal run only needs its two ends.
            [.., a, b] if a.1 == value && b.1 == value => b.0 = point.0,
            _ => self.points.push(point),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FROM: f64 = 1_000_000.0;
    const TO: f64 = FROM + 3600.0;
    const SLOT: f64 = 30.0;

    fn held(first: f64, last: f64, peak: f64, last_value: f64) -> MetricSlot {
        MetricSlot {
            first: FROM + first,
            last: FROM + last,
            peak,
            first_value: peak,
            last_value,
            last_climbing: false,
            ramp: false,
        }
    }

    /// A slot whose samples all climb from a head that started waiting 30s before `FROM`.
    fn climbing(first: f64, last: f64) -> MetricSlot {
        MetricSlot {
            first: FROM + first,
            last: FROM + last,
            peak: last + 30.0,
            first_value: first + 30.0,
            last_value: last + 30.0,
            last_climbing: true,
            ramp: true,
        }
    }

    fn at(secs: f64, value: f64) -> (i64, f64) {
        (((FROM + secs) * 1000.0) as i64, value)
    }

    #[test]
    fn a_value_holds_until_the_next_sample_and_a_drain_drops_where_it_was_written() {
        let line = render_series(
            None,
            &[
                held(60.0, 60.0, 3.0, 3.0),
                held(600.0, 600.0, 2.0, 2.0),
                held(900.0, 900.0, 0.0, 0.0),
            ],
            FROM,
            TO,
            SLOT,
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
        let line = render_series(None, &[held(60.0, 60.0, 3.0, 3.0)], FROM, TO, SLOT);
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
        // Samples at 60 (5), 70 (9), 80 (4) collapsed into one slot.
        let line = render_series(
            Some(&held(-30.0, -30.0, 2.0, 2.0)),
            &[held(60.0, 80.0, 9.0, 4.0)],
            FROM,
            FROM + 120.0,
            SLOT,
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

    #[test]
    fn a_climbing_delay_is_drawn_exactly_up_to_its_drain() {
        // 300s slots: one holds two climbing samples, and heartbeats follow until the drain.
        let line = render_series(
            None,
            &[
                climbing(60.0, 120.0),
                climbing(360.0, 360.0),
                climbing(660.0, 660.0),
                held(900.0, 900.0, 0.0, 0.0),
            ],
            FROM,
            TO,
            300.0,
        );
        assert_eq!(
            line,
            vec![
                at(60.0, 0.0),
                // The slot is one climb, not its peak held across it.
                at(60.0, 90.0),
                at(120.0, 150.0),
                // A vertex at each slot boundary the climb crosses.
                at(300.0, 330.0),
                at(360.0, 390.0),
                at(600.0, 630.0),
                at(660.0, 690.0),
                // Still climbing right up to the closing zero.
                at(900.0, 930.0),
                at(900.0, 0.0),
                at(3600.0, 0.0),
            ]
        );
    }
}
