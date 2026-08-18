//! OSS fallback for the pipeline freshness watchdog. The active backstop —
//! re-running a `// freshness`-annotated producer whose output aged past its
//! window — is an enterprise feature (see `freshness_watchdog_ee`). In the
//! public build the tick is a no-op; CE keeps the passive fresh/stale badge
//! on the asset graph.

use windmill_common::DB;

pub async fn tick(_db: &DB) {}
