use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use futures::StreamExt;
use lapin::options::{BasicAckOptions, BasicNackOptions};
use tokio::sync::RwLock;
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    jobs::JobTriggerKind,
    worker::to_raw_value,
    DB,
};

use windmill_store::resources::try_get_resource_from_db_as;
use windmill_trigger::listener::ListeningTrigger;
use windmill_trigger::trigger_helpers::TriggerJobArgs;
use windmill_trigger::Listener;

use super::{AmqpClientBuilder, AmqpConfig, AmqpConsumer, AmqpResource, AmqpTrigger};

#[async_trait]
impl Listener for AmqpTrigger {
    type Consumer = AmqpConsumer;
    type Extra = ();
    type ExtraState = ();
    const JOB_TRIGGER_KIND: JobTriggerKind = JobTriggerKind::Amqp;

    async fn get_consumer(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        _err_message: Arc<RwLock<Option<String>>>,
        _killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<Option<Self::Consumer>> {
        let ListeningTrigger::<Self::TriggerConfig> { workspace_id, trigger_config, .. } =
            listening_trigger;

        let AmqpConfig { amqp_resource_path, queue_name, exchange, options } = trigger_config;

        let authed = listening_trigger
            .authed(db, &Self::TRIGGER_KIND.to_string())
            .await?;

        let amqp_resource = try_get_resource_from_db_as::<AmqpResource>(
            &authed,
            Some(UserDB::new(db.clone())),
            db,
            amqp_resource_path,
            workspace_id,
        )
        .await?;

        let client_builder = AmqpClientBuilder::new(
            amqp_resource,
            queue_name,
            exchange.as_ref().map(|e| &e.0),
            options.as_ref().map(|o| &o.0),
        );

        let consumer = client_builder
            .build_consumer()
            .await
            .map_err(|e| Error::BadConfig(format!("Failed to build AMQP consumer: {}", e)))?;

        Ok(Some(consumer))
    }

    async fn consume(
        &self,
        db: &DB,
        mut consumer: Self::Consumer,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        _err_message: Arc<RwLock<Option<String>>>,
        _killpill_rx: tokio::sync::broadcast::Receiver<()>,
        _extra_state: Option<&Self::ExtraState>,
    ) {
        tracing::info!(
            "Starting to listen for AMQP trigger {}",
            &listening_trigger.path
        );

        // `consumer` owns the connection and channel, keeping them alive for the
        // whole loop; dropping either would tear down the AMQP subscription.
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
                    // (e.g. a transient DB error) nack with requeue so the broker
                    // redelivers rather than dropping the message (at-least-once).
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
                        // Ack/nack IO error: the channel is gone. Stop; the listener
                        // framework re-lists this trigger and reconnects.
                        tracing::warn!(
                            "Error acknowledging AMQP message for trigger {}: {}",
                            &listening_trigger.path,
                            err
                        );
                        return;
                    }

                    if dispatch_failed {
                        // The message was requeued. Stop consuming instead of
                        // immediately polling again: RabbitMQ redelivers the requeued
                        // message right away, so continuing would spin a tight retry
                        // loop that hammers job dispatch and error reporting. The
                        // framework re-lists the trigger after its ping goes stale
                        // (~15s), which backs the redelivery off to that cadence.
                        return;
                    }
                }
                Some(Err(err)) => {
                    // Connection/consumer error. Don't permanently disable on a
                    // transient blip (lapin has no built-in reconnect): stop and let
                    // the framework re-list and reconnect. A persistent failure is
                    // caught by `get_consumer`, which disables the trigger there.
                    tracing::warn!(
                        "AMQP consumer error for trigger {}, will reconnect: {}",
                        &listening_trigger.path,
                        err
                    );
                    return;
                }
                None => {
                    tracing::warn!(
                        "AMQP consumer stream ended for trigger {}, will reconnect",
                        &listening_trigger.path
                    );
                    return;
                }
            }
        }
    }
}
