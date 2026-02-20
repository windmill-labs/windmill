pub mod more_serde;
pub mod s3;

#[cfg(not(target_arch = "wasm32"))]
pub mod apps;
#[cfg(not(target_arch = "wasm32"))]
pub mod assets;
#[cfg(not(target_arch = "wasm32"))]
pub mod flow_status;
#[cfg(not(target_arch = "wasm32"))]
pub mod flows;
#[cfg(not(target_arch = "wasm32"))]
pub mod jobs;
#[cfg(not(target_arch = "wasm32"))]
pub mod runnable_settings;
#[cfg(not(target_arch = "wasm32"))]
pub mod schedule;
#[cfg(not(target_arch = "wasm32"))]
pub mod scripts;
#[cfg(not(target_arch = "wasm32"))]
pub mod triggers;

/// Duplicated from windmill-common::worker::to_raw_value.
/// windmill-types cannot depend on windmill-common (it would be circular).
pub fn to_raw_value<T: serde::Serialize>(result: &T) -> Box<serde_json::value::RawValue> {
    serde_json::value::to_raw_value(result)
        .unwrap_or_else(|_| serde_json::value::RawValue::from_string("{}".to_string()).unwrap())
}
