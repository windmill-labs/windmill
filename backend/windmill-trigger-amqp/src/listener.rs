use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures::StreamExt;
use lapin::options::{BasicAckOptions, BasicNackOptions};
use tokio::sync::RwLock;
use windmill_common::{
    db::UserDB,
    error::Result,
    jobs::JobTriggerKind,
    worker::to_raw_value,
    DB,
};

use windmill_store::resources::try_get_resource_from_db_as;
use windmill_trigger::listener::{transient_error, update_rw_lock, ListeningTrigger};
use windmill_trigger::trigger_helpers::TriggerJobArgs;
use windmill_trigger::Listener;

use super::{AmqpClientBuilder, AmqpConfig, AmqpConsumer, AmqpResource, AmqpTrigger};

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

        // lapin has no transparent reconnect: a broker outage surfaces here, retried by
        // the listener loop like any other failed connection.
        client_builder
            .build_consumer()
            .await
            .map_err(|e| transient_error(format!("Failed to build AMQP consumer: {}", e)))
    }
}

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
        self.build_amqp_consumer(db, listening_trigger)
            .await
            .map(Some)
    }

    async fn consume(
        &self,
        db: &DB,
        mut consumer: Self::Consumer,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        err_message: Arc<RwLock<Option<String>>>,
        _killpill_rx: tokio::sync::broadcast::Receiver<()>,
        _extra_state: Option<&Self::ExtraState>,
    ) {
        let path = &listening_trigger.path;
        let status = loop {
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
                        tracing::warn!(
                            "AMQP trigger {} ack/nack failed, reconnecting: {}",
                            path,
                            err
                        );
                        break format!("Acknowledging a message failed: {}", err);
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
                    break format!("Consumer error: {}", err);
                }
                None => {
                    tracing::warn!("AMQP trigger {} consumer stream ended, reconnecting", path);
                    break "Consumer stream ended".to_string();
                }
            }
        };
        update_rw_lock(err_message, Some(status)).await;
    }
}
