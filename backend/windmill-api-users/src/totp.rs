//! Native TOTP second factor for password logins. The implementation is in `totp_ee`.

#[cfg(feature = "private")]
pub use crate::totp_ee::{
    begin_setup, check_login_code, finish_setup, notify, service, TotpChange,
};
#[cfg(not(feature = "private"))]
pub use crate::totp_oss::{
    begin_setup, check_login_code, finish_setup, notify, service, TotpChange,
};

pub const TOTP_REQUIRED: &str = "Two-factor code required";
pub const TOTP_INVALID: &str = "Invalid two-factor code";
pub const TOTP_SETUP_REQUIRED: &str = "Two-factor setup required";

#[derive(serde::Serialize)]
pub struct TotpSetup {
    pub secret: String,
    pub otpauth_url: String,
}

/// Whether every password login must have a second factor. Enterprise only: a build without
/// it lets a single account enroll, so it could not satisfy the requirement.
pub fn required() -> bool {
    cfg!(all(feature = "enterprise", feature = "private"))
        && windmill_common::global_settings::REQUIRE_TOTP.load(std::sync::atomic::Ordering::Relaxed)
}
