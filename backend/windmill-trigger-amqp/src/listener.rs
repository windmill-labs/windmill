use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures::StreamExt;
use lapin::options::{BasicAckOptions, BasicNackOptions};
use tokio::sync::RwLock;
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    jobs::JobTriggerKind,
    utils::{report_critical_error, report_recovered_critical_error},
    worker::to_raw_value,
    DB,
};

use windmill_store::resources::try_get_resource_from_db_as;
use windmill_trigger::listener::ListeningTrigger;
use windmill_trigger::trigger_helpers::TriggerJobArgs;
use windmill_trigger::Listener;

use super::{AmqpClientBuilder, AmqpConfig, AmqpConsumer, AmqpResource, AmqpTrigger};

// lapin (like rdkafka) has no transparent reconnect, so — mirroring the Kafka
// trigger — the connection is (re)established in `consume` with a backoff retry
// loop rather than disabling the trigger on a transient broker outage.
const RECONNECT_BACKOFF_SECS: u64 = 30;
// Back off after a failed dispatch so a poison message that always fails can't
// spin a tight redelivery loop; the connection stays up for other messages.
const DISPATCH_FAILURE_BACKOFF_SECS: u64 = 5;

impl AmqpTrigger {
    async fn build_amqp_consumer(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<AmqpConfig>,
    ) -> Result<AmqpConsumer> {
        let AmqpConfig { amqp_resource_path, queue_name, exchange, options } =
            &listening_trigger.trigger_config;

        let authed = listening_trigger
            .authed(db, &Self::TRIGGER_KIND.to_string())
            .await?;

        let amqp_resource = try_get_resource_from_db_as::<AmqpResource>(
            &authed,
            Some(UserDB::new(db.clone())),
            db,
            amqp_resource_path,
            &listening_trigger.workspace_id,
        )
        .await?;

        let client_builder = AmqpClientBuilder::new(
            amqp_resource,
            queue_name,
            exchange.as_ref().map(|e| &e.0),
            options.as_ref().map(|o| &o.0),
        );

        client_builder
            .build_consumer()
            .await
            .map_err(|e| Error::BadConfig(format!("Failed to build AMQP consumer: {}", e)))
    }
}

#[async_trait]
impl Listener for AmqpTrigger {
    type Consumer = ();
    type Extra = ();
    type ExtraState = ();
    const JOB_TRIGGER_KIND: JobTriggerKind = JobTriggerKind::Amqp;

    async fn get_consumer(
        &self,
        _db: &DB,
        _listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        _err_message: Arc<RwLock<Option<String>>>,
        _killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<Option<Self::Consumer>> {
        // The connection is established (and re-established) in `consume` so that
        // a transient broker outage retries with backoff instead of disabling the
        // trigger — mirroring the Kafka trigger, whose client also lacks a
        // transparent reconnect.
        Ok(Some(()))
    }

    async fn consume(
        &self,
        db: &DB,
        _consumer: Self::Consumer,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        err_message: Arc<RwLock<Option<String>>>,
        _killpill_rx: tokio::sync::broadcast::Receiver<()>,
        _extra_state: Option<&Self::ExtraState>,
    ) {
        let path = &listening_trigger.path;
        let workspace_id = &listening_trigger.workspace_id;
        let alert_id = format!("amqp_trigger:{}", path);
        let mut tries = 0_usize;

        // (Re)connect loop: retries forever with backoff; the framework's
        // `select!` around `consume` cancels it on killpill, and
        // `update_ping_and_loop_ping_status` returning None (trigger removed /
        // disabled / capture stopped) breaks us out.
        loop {
            let mut consumer = match self.build_amqp_consumer(db, listening_trigger).await {
                Ok(consumer) => consumer,
                Err(e) => {
                    let status = format!(
                        "Failed to connect (attempt {}), retrying in {}s: {}",
                        tries + 1,
                        RECONNECT_BACKOFF_SECS,
                        e
                    );
                    if self
                        .update_ping_and_loop_ping_status(
                            db,
                            listening_trigger,
                            err_message.clone(),
                            Some(status),
                        )
                        .await
                        .is_none()
                    {
                        return;
                    }
                    tracing::error!(
                        "AMQP trigger {} failed to connect (attempt {}): {}",
                        path,
                        tries + 1,
                        e
                    );
                    if tries % 10 == 0 && listening_trigger.trigger_mode {
                        report_critical_error(
                            format!(
                                "Failed to connect AMQP trigger {} (attempt {}), retrying every {}s. This alert repeats every 10 failed attempts. Error: {}",
                                path, tries + 1, RECONNECT_BACKOFF_SECS, e
                            ),
                            db.clone(),
                            Some(workspace_id),
                            Some(&alert_id),
                        )
                        .await;
                    }
                    tries += 1;
                    tokio::time::sleep(Duration::from_secs(RECONNECT_BACKOFF_SECS)).await;
                    continue;
                }
            };

            // Connected: clear any "reconnecting" status.
            if self
                .update_ping_and_loop_ping_status(db, listening_trigger, err_message.clone(), None)
                .await
                .is_none()
            {
                return;
            }
            if tries > 0 {
                tracing::info!("AMQP trigger {} reconnected after {} attempts", path, tries);
                if listening_trigger.trigger_mode {
                    report_recovered_critical_error(
                        format!("AMQP trigger {} reconnected", path),
                        db.clone(),
                        Some(workspace_id),
                        Some(&alert_id),
                    )
                    .await;
                }
                tries = 0;
            }

            // Consume until the stream errors, then break out to reconnect.
            loop {
                match consumer.consumer.next().await {
                    Some(Ok(delivery)) => {
                        let trigger_info = HashMap::from([
                            (
                                "exchange".to_string(),
                                to_raw_value(&delivery.exchange.as_str()),
                            ),
                            (
                                "routing_key".to_string(),
                                to_raw_value(&delivery.routing_key.as_str()),
                            ),
                            ("queue_name".to_string(), to_raw_value(&consumer.queue_name)),
                            (
                                "redelivered".to_string(),
                                to_raw_value(&delivery.redelivered),
                            ),
                            (
                                "delivery_tag".to_string(),
                                to_raw_value(&delivery.delivery_tag),
                            ),
                        ]);

                        let dispatched = self
                            .handle_event(
                                db,
                                listening_trigger,
                                delivery.data.clone(),
                                trigger_info,
                                None,
                            )
                            .await;

                        // Only ack once the job/capture was dispatched. On failure
                        // nack with requeue so the broker redelivers rather than
                        // dropping the message (at-least-once).
                        let dispatch_failed = dispatched.is_err();
                        let ack_result = if dispatch_failed {
                            delivery
                                .acker
                                .nack(BasicNackOptions { requeue: true, multiple: false })
                                .await
                        } else {
                            delivery.acker.ack(BasicAckOptions::default()).await
                        };

                        if let Err(err) = ack_result {
                            // Channel is gone; break out to reconnect.
                            tracing::warn!(
                                "AMQP trigger {} ack/nack failed, reconnecting: {}",
                                path,
                                err
                            );
                            break;
                        }

                        if dispatch_failed {
                            // The message was requeued: back off before consuming
                            // again so a poison message can't spin a tight
                            // redelivery loop, while keeping the connection alive.
                            tokio::time::sleep(Duration::from_secs(DISPATCH_FAILURE_BACKOFF_SECS))
                                .await;
                        }
                    }
                    Some(Err(err)) => {
                        tracing::warn!(
                            "AMQP trigger {} consumer error, reconnecting: {}",
                            path,
                            err
                        );
                        break;
                    }
                    None => {
                        tracing::warn!("AMQP trigger {} consumer stream ended, reconnecting", path);
                        break;
                    }
                }
            }
        }
    }
}
