use std::sync::atomic::{AtomicU64, Ordering};

static INSTANCE_AI_CONFIG_REVISION: AtomicU64 = AtomicU64::new(0);

pub fn current_instance_ai_config_revision() -> u64 {
    INSTANCE_AI_CONFIG_REVISION.load(Ordering::SeqCst)
}

pub fn bump_instance_ai_config_revision() -> u64 {
    INSTANCE_AI_CONFIG_REVISION.fetch_add(1, Ordering::SeqCst) + 1
}
