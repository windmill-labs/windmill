#[allow(unused)]
#[cfg(feature = "private")]
pub use super::listener_ee::*;

#[cfg(not(feature = "private"))]
use {
    super::NatsTrigger,
    serde_json::value::RawValue,
    std::{collections::HashMap, sync::Arc},
    tokio::sync::RwLock,
    windmill_common::{error::Result, jobs::JobTriggerKind, triggers::TriggerKind, DB},
    windmill_trigger::{listener::ListeningTrigger, trigger_helpers::TriggerJobArgs, Listener},
};

#[cfg(not(feature = "private"))]
impl TriggerJobArgs for NatsTrigger {
    type Payload = Vec<u8>;
    const TRIGGER_KIND: TriggerKind = TriggerKind::Nats;
    fn v1_payload_fn(_payload: &Self::Payload) -> HashMap<String, Box<RawValue>> {
        HashMap::new()
    }
}

#[cfg(not(feature = "private"))]
#[async_trait::async_trait]
impl Listener for NatsTrigger {
    type Consumer = ();
    type Extra = ();
    type ExtraState = ();
    const JOB_TRIGGER_KIND: JobTriggerKind = JobTriggerKind::Nats;

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
        _extra_state: Option<&Self::ExtraState>,
    ) {
        ()
    }
}
