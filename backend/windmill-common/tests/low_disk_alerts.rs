//! Regression test for the server-mode low-disk alert dedup tag.
//!
//! ## Requirements
//!
//! - PostgreSQL database running locally
//! - Enterprise features enabled
//!
//! ## Running the tests
//!
//! ```bash
//! cargo test -p windmill-common --test low_disk_alerts --features private,enterprise -- --ignored --nocapture
//! ```

#[cfg(all(feature = "private", feature = "enterprise"))]
mod tests {
    use sqlx::{Pool, Postgres};
    use windmill_common::ee::low_disk_alerts;
    use windmill_common::utils::HOSTNAME;

    /// The server tag must carry the hostname: `simple_alert_helper` keys one alert row per
    /// tag, so a host-less tag lets a replica seeing low disk and a replica seeing free disk
    /// raise and recover the same row every monitor pass.
    ///
    /// The hostname is forced to a pod-length name so the tag runs past 50 chars, which
    /// `check_type` must stay wide enough to hold: `create_alert` only logs the insert
    /// error while the notification still fires, so a tag that does not fit re-alerts every
    /// pass and never recovers. Asserting the row persists pins the width and the shape.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn server_low_disk_tag_is_per_host(db: Pool<Postgres>) {
        // Both statics are lazy and read on first access inside the call below.
        std::env::set_var("FORCE_HOSTNAME", "windmill-server-7d9f8b6c4d-x2k9p");
        // Force every mount to read as low so the server branch raises.
        std::env::set_var("MIN_FREE_DISK_SPACE_MB", "999999999999");

        low_disk_alerts(&db, true, false, vec![]).await;

        let tags: Vec<String> = sqlx::query_scalar(
            "SELECT check_type FROM healthchecks WHERE check_type LIKE 'low-disk-v2-server@%'",
        )
        .fetch_all(&db)
        .await
        .unwrap();

        assert!(
            !tags.is_empty(),
            "expected at least one server low-disk alert; an alert whose tag does not fit \
             check_type is dropped here while its notification still fires"
        );
        for tag in &tags {
            assert!(
                tag.ends_with(&format!("@{}", *HOSTNAME)),
                "server tag {tag} is not per-host; replicas would share one alert row"
            );
        }
    }
}
