use super::{
    Encoding, HmacAlgorithm, HmacAuthenticationData, HmacAuthenticationDetails,
    TryGetWebhookHeader, WebhookAuthenticationMethod, WebhookError, WebhookHandler,
};
use axum::response::Response;
use http::HeaderMap;
use std::borrow::Cow;

pub struct TikTok;

impl WebhookHandler for TikTok {
    fn handle_challenge_request<'header>(
        &self,
        _: &'header HeaderMap,
        _: &WebhookAuthenticationMethod,
        _: &str,
    ) -> Result<Option<Response>, WebhookError> {
        Ok(None)
    }

    fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &'payload str,
    ) -> Result<HmacAuthenticationData<'payload, 'header, 'prefix>, WebhookError> {
        let tiktok_secret_signature = headers.try_get_webhook_header("TikTok-Signature")?;

        let tiktok_signature = TikTokSignature::parse(tiktok_secret_signature)?;
        let signed_payload = format!("{}.{}", tiktok_signature.t, raw_payload);

        Ok(HmacAuthenticationData::new(
            Cow::Owned(signed_payload),
            tiktok_signature.s,
            None,
            HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex),
        ))
    }
}

#[derive(Debug)]
pub struct TikTokSignature<'r> {
    pub t: i64,
    pub s: &'r str,
}

impl<'r> TikTokSignature<'r> {
    pub fn parse(raw: &'r str) -> Result<TikTokSignature<'r>, WebhookError> {
        use std::collections::HashMap;
        let headers: HashMap<&str, &str> = raw
            .split(',')
            .map(|header| {
                let mut key_and_value = header.split('=');
                let key = key_and_value.next();
                let value = key_and_value.next();
                (key, value)
            })
            .filter_map(|(key, value)| match (key, value) {
                (Some(key), Some(value)) => Some((key, value)),
                _ => None,
            })
            .collect();
        let t = headers.get("t").ok_or(WebhookError::InvalidSignature)?;
        let s = headers.get("s").ok_or(WebhookError::InvalidSignature)?;
        Ok(TikTokSignature {
            t: t.parse::<i64>()
                .map_err(|_| WebhookError::InvalidTimestamp)?,
            s,
        })
    }
}
