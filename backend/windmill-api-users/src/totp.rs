//! Native TOTP second factor for password logins. The implementation is in `totp_ee`.

#[cfg(feature = "private")]
pub use crate::totp_ee::{check_login_code, service};
#[cfg(not(feature = "private"))]
pub use crate::totp_oss::{check_login_code, service};

pub const TOTP_REQUIRED: &str = "Two-factor code required";
pub const TOTP_INVALID: &str = "Invalid two-factor code";
