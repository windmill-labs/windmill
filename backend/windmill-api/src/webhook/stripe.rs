use super::{
    Encoding, HmacAlgorithm, HmacAuthenticationData, HmacAuthenticationDetails,
    TryGetWebhookHeader, WebhookAuthenticationMethod, WebhookError, WebhookHandler,
};
use axum::response::Response;
use http::HeaderMap;
use std::borrow::Cow;

pub struct Stripe;

impl WebhookHandler for Stripe {
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
        let stripe_signature_header = headers.try_get_webhook_header("STRIPE-SIGNATURE")?;

        let stripe_signature = StripeSignature::parse(stripe_signature_header)?;
        let signed_payload = format!("{}.{}", stripe_signature.t, raw_payload);

        Ok(HmacAuthenticationData::new(
            Cow::Owned(signed_payload),
            stripe_signature.v1,
            None,
            HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex),
        ))
    }
}

#[derive(Debug)]
pub struct StripeSignature<'header> {
    pub t: i64,
    pub v1: &'header str,
    #[allow(unused)]
    pub v0: Option<&'header str>,
}

impl<'header> StripeSignature<'header> {
    pub fn parse(raw: &'header str) -> Result<StripeSignature<'header>, WebhookError> {
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
        let v1 = headers.get("v1").ok_or(WebhookError::InvalidSignature)?;
        let v0 = headers.get("v0").map(|r| *r);
        Ok(StripeSignature {
            t: t.parse::<i64>()
                .map_err(|_| WebhookError::InvalidTimestamp)?,
            v1,
            v0,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_signature_parse() {
        use super::StripeSignature;

        let raw_signature =
            "t=1492774577,v1=5257a869e7ecebeda32affa62cdca3fa51cad7e77a0e56ff536d0ce8e108d8bd";
        let signature = StripeSignature::parse(raw_signature).unwrap();
        assert_eq!(signature.t, 1492774577);
        assert_eq!(
            signature.v1,
            "5257a869e7ecebeda32affa62cdca3fa51cad7e77a0e56ff536d0ce8e108d8bd"
        );
        assert_eq!(signature.v0, None);

        let raw_signature_with_test_mode = "t=1492774577,v1=5257a869e7ecebeda32affa62cdca3fa51cad7e77a0e56ff536d0ce8e108d8bd,v0=6ffbb59b2300aae63f272406069a9788598b792a944a07aba816edb039989a39";
        let signature = StripeSignature::parse(raw_signature_with_test_mode).unwrap();
        assert_eq!(signature.t, 1492774577);
        assert_eq!(
            signature.v1,
            "5257a869e7ecebeda32affa62cdca3fa51cad7e77a0e56ff536d0ce8e108d8bd"
        );
        assert_eq!(
            signature.v0,
            Some("6ffbb59b2300aae63f272406069a9788598b792a944a07aba816edb039989a39")
        );
    }
}
