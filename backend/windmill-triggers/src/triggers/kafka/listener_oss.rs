#[allow(unused)]

#[cfg(feature = "private")]
pub use super::listener_ee::*;

#[cfg(not(feature = "private"))]
use {
    super::KafkaTrigger,
    crate::triggers::{listener::ListeningTrigger, Listener},
    std::sync::Arc,
    tokio::sync::RwLock,
    windmill_common::{error::Result, jobs::JobTriggerKind, DB},
};

#[cfg(not(feature = "private"))]
#[async_trait::async_trait]
impl Listener for KafkaTrigger {
    type Consumer = ();
    type Extra = ();
    const JOB_TRIGGER_KIND: JobTriggerKind = JobTriggerKind::Kafka;

    async fn get_consumer(
        &self,
        _db: &DB,
        _listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        _err_message: Arc<RwLock<Option<String>>>,
        _killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<Option<Self::Consumer>> {
        Ok(None)
    }
    async fn consume(
        &self,
        _db: &DB,
        _consumer: Self::Consumer,
        _listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        _err_message: Arc<RwLock<Option<String>>>,
        _killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) {
        ()
    }
}
