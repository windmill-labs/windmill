use dashmap::DashMap;
use std::borrow::Borrow;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};

/// Calls between eviction sweeps.
const EVICTION_INTERVAL: u64 = 256;

struct Bucket {
    count: u32,
    minute: i64,
}

/// Events recorded per key within the current wall-clock minute, held in process memory.
///
/// Counts are per process and reset on restart, so N servers raise any threshold built on
/// this to N times its configured value. Stale keys are swept as calls come in, which keeps the
/// map bounded without a background task.
pub struct PerMinuteCounter<K: Eq + Hash> {
    buckets: DashMap<K, Bucket>,
    calls: AtomicU64,
}

impl<K: Eq + Hash> PerMinuteCounter<K> {
    pub fn new() -> Self {
        Self { buckets: DashMap::new(), calls: AtomicU64::new(0) }
    }

    /// Events recorded for `key` in the current minute, without recording one. A key never
    /// seen, or last seen in an earlier minute, counts as zero.
    pub fn count<Q>(&self, key: &Q) -> u32
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let minute = current_minute();
        self.buckets
            .get(key)
            .filter(|bucket| bucket.minute == minute)
            .map_or(0, |bucket| bucket.count)
    }

    /// Record one event and return the new count for the current minute.
    pub fn increment(&self, key: K) -> u32 {
        self.bump_at(key, None, current_minute()).0
    }

    /// Record one event unless `key` has already reached `limit` this minute. Returns false
    /// when the limit was already reached, in which case nothing was recorded.
    pub fn try_increment(&self, key: K, limit: u32) -> bool {
        self.bump_at(key, Some(limit), current_minute()).1
    }

    /// Returns the count for `minute` and whether this call recorded an event. Takes the
    /// minute rather than reading the clock so the rollover and eviction paths are testable.
    fn bump_at(&self, key: K, limit: Option<u32>, minute: i64) -> (u32, bool) {
        // The entry guard holds a lock on its DashMap shard, and the `retain` below takes
        // every shard. Keeping the guard alive across that call deadlocks the caller, so this
        // block is load-bearing: it must end before the sweep, not be flattened into the body.
        let outcome = {
            let mut bucket = self
                .buckets
                .entry(key)
                .or_insert(Bucket { count: 0, minute });
            if bucket.minute != minute {
                bucket.minute = minute;
                bucket.count = 0;
            }
            if limit.is_some_and(|limit| bucket.count >= limit) {
                (bucket.count, false)
            } else {
                bucket.count += 1;
                (bucket.count, true)
            }
        };
        // Periodically drop what neither the current nor the previous minute can still need.
        // Counts every call, refusals included: a key pinned at its limit must still drive
        // sweeps, or a sustained burst of refusals would leave the map unbounded.
        if self.calls.fetch_add(1, Ordering::Relaxed) % EVICTION_INTERVAL == 0 {
            self.buckets.retain(|_, bucket| bucket.minute >= minute - 1);
        }
        outcome
    }
}

fn current_minute() -> i64 {
    chrono::Utc::now().timestamp() / 60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_at_the_limit_without_recording() {
        let counter = PerMinuteCounter::new();
        assert_eq!(counter.bump_at("a".to_string(), Some(2), 100), (1, true));
        assert_eq!(counter.bump_at("a".to_string(), Some(2), 100), (2, true));
        // A refused call must not record, or a caller held at the limit would keep inflating
        // its own count and never recover within the minute.
        assert_eq!(counter.bump_at("a".to_string(), Some(2), 100), (2, false));
        assert_eq!(counter.bump_at("a".to_string(), Some(2), 100), (2, false));
    }

    #[test]
    fn resets_on_the_next_minute() {
        let counter = PerMinuteCounter::new();
        counter.bump_at("a".to_string(), None, 100);
        counter.bump_at("a".to_string(), None, 100);
        assert_eq!(counter.bump_at("a".to_string(), None, 101), (1, true));
    }

    #[test]
    fn sweep_drops_keys_older_than_the_previous_minute() {
        let counter = PerMinuteCounter::new();
        counter.bump_at("stale".to_string(), None, 100);
        counter.bump_at("previous".to_string(), None, 199);
        // Sweeps fire every EVICTION_INTERVAL calls; drive the counter to the next one with
        // filler events that all land in minute 200.
        while counter.calls.load(Ordering::Relaxed) <= EVICTION_INTERVAL {
            counter.bump_at("filler".to_string(), None, 200);
        }
        assert!(!counter.buckets.contains_key("stale"));
        assert!(counter.buckets.contains_key("previous"));
        assert!(counter.buckets.contains_key("filler"));
    }
}
