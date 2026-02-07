#[cfg(feature = "private")]
mod handler_ee;
pub mod handler_oss;

#[cfg(feature = "private")]
mod listener_ee;
pub mod listener_oss;

#[cfg(feature = "private")]
mod mod_ee;
#[cfg(feature = "private")]
pub use mod_ee::*;

#[derive(Copy, Clone)]
pub struct NatsTrigger;
