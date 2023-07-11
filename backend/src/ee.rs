#[cfg(feature = "enterprise")]
use base64::Engine;
#[cfg(feature = "enterprise")]
use rsa::{pkcs8::DecodePublicKey, signature::Verifier};
#[cfg(feature = "enterprise")]
use sha2::Sha256;

#[cfg(feature = "enterprise")]
pub fn verify_license_key(license_key: Option<String>) -> anyhow::Result<()> {
    if let Some(license_key) = license_key {
        let mut splitted_lk = license_key.split(".");
        if splitted_lk.clone().count() != 3 {
            panic!("license_key can be splitted with 2 . (<client id>.<expiry>.<signature>)");
        }
        let id = splitted_lk.next().unwrap();
        let expiry = splitted_lk.next().unwrap();
        let signature_b64 = splitted_lk.next().unwrap();

        let expiry_nb = expiry.parse::<u64>()?;
        if expiry_nb < chrono::Utc::now().timestamp() as u64 {
            panic!(
                "License key is expired (timestamp expiry: {expiry_nb}. Now: {}",
                chrono::Utc::now().timestamp()
            );
        }
        const PUBLIC_KEY: &str = "MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDgVShzcLSPiOi+8ET8fggob1kmi47/cE12JaidPkwfGnScZItghkqtiLsct0U4kJhlp5gO89DYTBmIKadvxwY7kMsLlZzmi2emVH7c27cByGASY8QmWDNdG4Ggy/NDflGGBdAtN6gHawZAg4zHv3qpbPQGHH1/6sXIohcXhOnouwIDAQAB";
        let pub_key = rsa::RsaPublicKey::from_public_key_der(
            &base64::engine::general_purpose::STANDARD.decode(PUBLIC_KEY)?,
        )?;
        let signature = base64::engine::general_purpose::STANDARD.decode(signature_b64)?;
        rsa::pss::VerifyingKey::<Sha256>::new(pub_key)
            .verify(
                &format!("{id}{expiry}").as_bytes(),
                &rsa::pss::Signature::from(signature),
            )
            .map_err(|_| anyhow::anyhow!("Invalid license key".to_string()))?;
    } else {
        panic!("License key is required for the enterprise edition");
    }
    Ok(())
}
