/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::Router;
use crate::db::DB;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use windmill_common::jobs::QueuedJob;

pub fn workspaced_service() -> Router {
    crate::gcp_triggers_ee::workspaced_service()
}

pub fn start_consuming_gcp_pubsub_event(
    db: DB,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    crate::gcp_triggers_ee::start_consuming_gcp_pubsub_event(db, killpill_rx)
}

pub fn gcp_push_route_handler() -> Router {
    crate::gcp_triggers_ee::gcp_push_route_handler()
}

pub async fn manage_google_subscription(
    path: String,
    trigger: GcpTrigger,
    operation: String,
    w_id: String,
    db: DB,
) -> anyhow::Result<()> {
    crate::gcp_triggers_ee::manage_google_subscription(path, trigger, operation, w_id, db).await
}

pub async fn process_google_push_request(
    workspace_id: String,
    trigger_token: String,
    message: HashMap<String, serde_json::Value>,
    db: DB,
) -> anyhow::Result<Option<QueuedJob>> {
    crate::gcp_triggers_ee::process_google_push_request(workspace_id, trigger_token, message, db).await
}

pub async fn validate_jwt_token(
    token: String,
    audience: String,
) -> anyhow::Result<()> {
    crate::gcp_triggers_ee::validate_jwt_token(token, audience).await
}

pub use crate::gcp_triggers_ee::CreateUpdateConfig;
pub use crate::gcp_triggers_ee::DeliveryType;
pub use crate::gcp_triggers_ee::ExistingGcpSubscription;
pub use crate::gcp_triggers_ee::GcpTrigger;
pub use crate::gcp_triggers_ee::PushConfig;
pub use crate::gcp_triggers_ee::SubscriptionMode;