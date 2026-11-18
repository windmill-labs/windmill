//! Runtime gate for the `Some(0)` concurrency footgun: a stored `concurrent_limit <= 0`
//! must read as "disabled", never as a zero-slot cap that permanently blocks the job at the
//! concurrency gate (the re-queue storm the zombie monitor eventually fails as a fake OOM).
//!
//! Run with:
//!   cargo test -p windmill-queue --test concurrency_limit_zero_test

use windmill_queue::jobs::has_active_concurrency_limit;

#[test]
fn zero_and_negative_are_not_active_limits() {
    assert!(!has_active_concurrency_limit(None));
    assert!(!has_active_concurrency_limit(Some(0)));
    assert!(!has_active_concurrency_limit(Some(-1)));
}

#[test]
fn positive_limit_is_active() {
    assert!(has_active_concurrency_limit(Some(1)));
    assert!(has_active_concurrency_limit(Some(i32::MAX)));
}
