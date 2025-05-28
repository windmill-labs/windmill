/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::Router;
use crate::db::DB;

pub fn workspaced_service() -> Router {
    crate::kafka_triggers_ee::workspaced_service()
}

pub fn start_kafka_consumers(
    db: DB,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    crate::kafka_triggers_ee::start_kafka_consumers(db, killpill_rx)
}

pub use crate::kafka_triggers_ee::KafkaResourceSecurity;
pub use crate::kafka_triggers_ee::KafkaTrigger;
pub use crate::kafka_triggers_ee::KafkaTriggerConfigConnection;