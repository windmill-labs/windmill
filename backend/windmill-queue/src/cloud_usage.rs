/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use chrono::Datelike;
use dashmap::DashMap;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct UsageKey {
    id: String,
    is_workspace: bool,
    month: i32,
}

pub struct UsageBuffer {
    buffer: Arc<DashMap<UsageKey, i32>>,
    db: Pool<Postgres>,
    shutdown_notify: Arc<Notify>,
}

impl UsageBuffer {
    pub fn new(db: Pool<Postgres>) -> Arc<Self> {
        let buffer = Arc::new(Self {
            buffer: Arc::new(DashMap::new()),
            db,
            shutdown_notify: Arc::new(Notify::new()),
        });

        // Spawn the periodic flush task
        let buffer_clone = buffer.clone();
        tokio::spawn(async move {
            buffer_clone.flush_loop().await;
        });

        buffer
    }

    pub fn increment(&self, workspace_id: String, email: Option<String>) {
        let month = Self::current_month();

        // Increment workspace usage
        self.buffer
            .entry(UsageKey { id: workspace_id, is_workspace: true, month })
            .and_modify(|counter| *counter += 1)
            .or_insert(1);

        // Increment user usage if email is provided
        if let Some(email) = email {
            self.buffer
                .entry(UsageKey { id: email, is_workspace: false, month })
                .and_modify(|counter| *counter += 1)
                .or_insert(1);
        }
    }

    async fn flush_loop(&self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.flush().await;
                }
                _ = self.shutdown_notify.notified() => {
                    // Final flush on shutdown
                    self.flush().await;
                    break;
                }
            }
        }
    }

    async fn flush(&self) {
        if self.buffer.is_empty() {
            return;
        }

        // Drain all buffered usage counts
        let mut to_flush = Vec::new();
        self.buffer.retain(|key, value| {
            to_flush.push((key.clone(), *value));
            false
        });

        if to_flush.is_empty() {
            return;
        }

        tracing::debug!(
            "Flushing {} buffered usage entries to database",
            to_flush.len()
        );

        // Batch update to database
        for (key, count) in to_flush {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                sqlx::query!(
                    "INSERT INTO usage (id, is_workspace, month_, usage)
                    VALUES ($1, $2, $3, $4)
                    ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + $4",
                    &key.id,
                    key.is_workspace,
                    key.month,
                    count
                )
                .execute(&self.db),
            )
            .await;

            match result {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    tracing::error!(
                        "Failed to flush usage for {} (is_workspace: {}): {:#}",
                        key.id,
                        key.is_workspace,
                        e
                    );
                }
                Err(_) => {
                    tracing::error!(
                        "Usage flush timed out for {} (is_workspace: {})",
                        key.id,
                        key.is_workspace
                    );
                }
            }
        }
    }

    fn current_month() -> i32 {
        let now = chrono::Utc::now();
        (now.year() * 12 + now.month() as i32) as i32
    }
}

lazy_static::lazy_static! {
    static ref USAGE_BUFFER: once_cell::sync::OnceCell<Arc<UsageBuffer>> = once_cell::sync::OnceCell::new();
}

pub fn init_usage_buffer(db: Pool<Postgres>) {
    USAGE_BUFFER.get_or_init(|| UsageBuffer::new(db));
}

pub fn increment_usage_async(db: Pool<Postgres>, workspace_id: String, email: Option<String>) {
    if let Some(buffer) = USAGE_BUFFER.get() {
        buffer.increment(workspace_id, email);
    } else {
        tracing::warn!("Usage buffer not initialized, falling back to direct database update");
        // Fallback to old implementation if buffer not initialized
        tokio::task::spawn(async move {
            let result = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                // Update workspace usage
                let workspace_result = sqlx::query!(
                    "INSERT INTO usage (id, is_workspace, month_, usage)
                    VALUES ($1, TRUE, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), 1)
                    ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + 1",
                    &workspace_id
                )
                .execute(&db)
                .await;

                if let Err(e) = workspace_result {
                    tracing::error!("Failed to update workspace usage for {}: {:#}", workspace_id, e);
                }

                // Update user usage if email is provided (non-premium workspaces only)
                if let Some(ref email) = email {
                    let user_result = sqlx::query!(
                        "INSERT INTO usage (id, is_workspace, month_, usage)
                        VALUES ($1, FALSE, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), 1)
                        ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + 1",
                        email
                    )
                    .execute(&db)
                    .await;

                    if let Err(e) = user_result {
                        tracing::error!("Failed to update user usage for {}: {:#}", email, e);
                    }
                }
            })
            .await;

            if let Err(_) = result {
                tracing::error!(
                    "Usage update timed out after 10s for workspace {} and email {:?}",
                    workspace_id,
                    email
                );
            }
        });
    }
}
