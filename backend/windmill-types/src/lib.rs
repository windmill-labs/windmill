pub mod apps;
pub mod assets;
pub mod flow_status;
pub mod flows;
pub mod jobs;
pub mod more_serde;
pub mod runnable_settings;
pub mod s3;
pub mod schedule;
pub mod scripts;
pub mod triggers;

/// Duplicated from windmill-common::worker::to_raw_value.
/// windmill-types cannot depend on windmill-common (it would be circular).
pub fn to_raw_value<T: serde::Serialize>(result: &T) -> Box<serde_json::value::RawValue> {
    serde_json::value::to_raw_value(result)
        .unwrap_or_else(|_| serde_json::value::RawValue::from_string("{}".to_string()).unwrap())
}
