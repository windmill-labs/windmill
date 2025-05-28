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
    crate::sqs_triggers_ee::workspaced_service()
}

pub fn start_sqs(
    db: DB,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    crate::sqs_triggers_ee::start_sqs(db, killpill_rx)
}

pub use crate::sqs_triggers_ee::SqsTrigger;