#[cfg(feature = "enterprise")]
use base64::Engine;
#[cfg(feature = "enterprise")]
use rsa::{pkcs8::DecodePublicKey, signature::Verifier};
#[cfg(feature = "enterprise")]
use sha2::Sha256;

#[cfg(feature = "enterprise")]
pub fn verify_license_key(license_key: Option<String>) -> anyhow::Result<()> {
    if let Some(license_key) = license_key {
        let splitted_lk = license_key
            .split_once(".")
            .expect("license_key can be splitted with a .");

        let pub_key = rsa::RsaPublicKey::from_public_key_der(
    &base64::engine::general_purpose::STANDARD.decode("MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDgVShzcLSPiOi+8ET8fggob1kmi47/cE12JaidPkwfGnScZItghkqtiLsct0U4kJhlp5gO89DYTBmIKadvxwY7kMsLlZzmi2emVH7c27cByGASY8QmWDNdG4Ggy/NDflGGBdAtN6gHawZAg4zHv3qpbPQGHH1/6sXIohcXhOnouwIDAQAB")?)?;
        let msg = base64::engine::general_purpose::STANDARD.decode(splitted_lk.0)?;
        let signature = base64::engine::general_purpose::STANDARD.decode(splitted_lk.1)?;
        rsa::pss::VerifyingKey::<Sha256>::new(pub_key)
            .verify(&msg, &rsa::pss::Signature::from(signature))
            .map_err(|_| anyhow::anyhow!("Invalid license key".to_string()))?;
    } else {
        panic!("License key is required for the enterprise edition");
    }
    Ok(())
}
