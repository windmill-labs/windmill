use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::queue_metrics::{read_queue_metrics_series, QUEUE_METRIC_STALE_SECS};

const WINDOW: f64 = 3600.0;

/// Store a sample written `at` seconds after the start of a `WINDOW` ending now.
async fn sample(db: &Pool<Postgres>, id: &str, value: serde_json::Value, at: f64) {
    sqlx::query(
        "INSERT INTO metrics (id, value, created_at) VALUES ($1, $2, now() - make_interval(secs => $3))",
    )
    .bind(id)
    .bind(value)
    .bind(WINDOW - at)
    .execute(db)
    .await
    .expect("failed to store a metric sample");
}

/// The database hands the renderer the last sample before the window, which sets the left edge,
/// and for each slot its peak and its latest value, which the line continues from.
#[sqlx::test(migrations = "../migrations")]
async fn a_series_starts_from_the_sample_before_the_window_and_keeps_each_slot_peak(
    db: Pool<Postgres>,
) {
    // Before the window: 1, then 2, which is what is in force at the left edge.
    sample(&db, "queue_count_t", json!(1), -120.0).await;
    sample(&db, "queue_count_t", json!(2), -60.0).await;
    // Three samples inside one 30s slot: the line rises to their peak, then drops to the last.
    sample(&db, "queue_count_t", json!(5), 605.0).await;
    sample(&db, "queue_count_t", json!(9), 612.0).await;
    sample(&db, "queue_count_t", json!(4), 620.0).await;
    // Drained before the window: nothing left to draw.
    sample(&db, "queue_count_gone", json!(3), -300.0).await;
    sample(&db, "queue_count_gone", json!(0), -200.0).await;

    let series = read_queue_metrics_series(&db, WINDOW).await.unwrap();

    assert_eq!(
        series.tags.len(),
        1,
        "a tag drained before the window is left out"
    );
    let tag = &series.tags[0];
    assert_eq!(tag.tag, "t");
    assert!(tag.delay.is_empty());

    let stale = 620.0 + QUEUE_METRIC_STALE_SECS;
    let expected = [
        (0.0, 2.0),
        (605.0, 2.0),
        (605.0, 9.0),
        (620.0, 9.0),
        (620.0, 4.0),
        (stale, 4.0),
        (stale, 0.0),
        (WINDOW, 0.0),
    ];
    assert_eq!(tag.count.len(), expected.len(), "vertices: {:?}", tag.count);
    for ((ms, value), (at, expected_value)) in tag.count.iter().zip(expected) {
        let secs = (*ms - series.from) as f64 / 1000.0;
        // Samples are stored a few milliseconds before the window is read.
        assert!(
            (secs - at).abs() < 2.0 && *value == expected_value,
            "expected ({at}, {expected_value}), got ({secs}, {value}) in {:?}",
            tag.count
        );
    }
}
