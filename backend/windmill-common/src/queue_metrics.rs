//! The queue metrics the monitor samples into `metrics` (`queue_count_{tag}` and
//! `queue_delay_{tag}`), and how a stored series is drawn back.

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
