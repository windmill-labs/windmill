use hmac::Hmac;
use sha2::Sha256;

pub type HmacSha256 = Hmac<Sha256>;
