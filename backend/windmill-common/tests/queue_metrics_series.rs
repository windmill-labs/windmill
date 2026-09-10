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

/// A delay stored as its head's wait start is drawn as that wait, growing a second per second,
/// right up to the zero that closes it.
#[sqlx::test(migrations = "../migrations")]
async fn a_climbing_delay_is_drawn_as_the_wait_of_its_head(db: Pool<Postgres>) {
    let now: f64 = sqlx::query_scalar("SELECT EXTRACT(EPOCH FROM now())::double precision")
        .fetch_one(&db)
        .await
        .unwrap();
    // The head started waiting 30s before the window; heartbeats restate it until the drain.
    let head = json!({ "since": now - WINDOW - 30.0 });
    for at in [60.0, 360.0, 660.0] {
        sample(&db, "queue_delay_t", head.clone(), at).await;
    }
    sample(&db, "queue_delay_t", json!(0), 900.0).await;

    let series = read_queue_metrics_series(&db, WINDOW).await.unwrap();
    let points = series.tags[0]
        .delay
        .iter()
        .map(|(ms, value)| ((*ms - series.from) as f64 / 1000.0, *value))
        .collect::<Vec<_>>();

    let climb = points
        .iter()
        .filter(|(_, value)| *value > 0.0)
        .collect::<Vec<_>>();
    assert!(
        climb.len() > 4,
        "the climb has vertices along the way: {points:?}"
    );
    for (at, value) in &climb {
        assert!(
            (value - (at + 30.0)).abs() < 2.0,
            "off the climb at {at}: {points:?}"
        );
    }
    let (first, _) = climb[0];
    let (top, _) = climb[climb.len() - 1];
    assert!(
        (first - 60.0).abs() < 2.0 && (top - 900.0).abs() < 2.0,
        "{points:?}"
    );
}
