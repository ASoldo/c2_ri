use std::time::{SystemTime, UNIX_EPOCH};

pub type EpochMillis = u64;

pub fn now_epoch_millis() -> EpochMillis {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as EpochMillis
}
