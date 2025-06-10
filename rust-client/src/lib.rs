#![doc = include_str!("../README.md")]
mod client;
mod maybe_future;
#[cfg(test)]
mod tests;

pub use client::{SdkError, Windmill};
pub use windmill_api::apis;
